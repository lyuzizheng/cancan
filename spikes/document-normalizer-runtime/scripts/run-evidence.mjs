import { createHash } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import {
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { createInterface } from "node:readline";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dist = join(root, "dist");
const metadata = JSON.parse(readFileSync(join(dist, "build-metadata.json"), "utf8"));
const sidecar = join(
  root,
  "src-tauri",
  "binaries",
  `cancan-document-agent-${metadata.target}`,
);

function withTimeout(promise, ms, message) {
  let timeoutId;
  const timeout = new Promise((_, reject) => {
    timeoutId = setTimeout(() => reject(new Error(message)), ms);
  });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timeoutId));
}

async function driveWorker(command, args) {
  const started = performance.now();
  const environmentSecret = "must-not-cross-worker-environment";
  const child = spawn(command, args, {
    cwd: root,
    env: { ...process.env, CANCAN_EVIDENCE_PROVIDER_KEY: environmentSecret },
    stdio: ["pipe", "pipe", "pipe"],
  });
  const messages = [];
  let stdout = "";
  let stderr = "";
  const waiters = [];
  createInterface({ input: child.stdout }).on("line", (line) => {
    stdout += `${line}\n`;
    const message = JSON.parse(line);
    messages.push(message);
    for (const waiter of [...waiters]) {
      if (waiter.predicate(message)) {
        waiter.resolve(message);
        waiters.splice(waiters.indexOf(waiter), 1);
      }
    }
  });
  child.stderr.on("data", (chunk) => {
    stderr += chunk.toString();
  });
  const waitFor = (predicate) =>
    withTimeout(
      new Promise((resolve) => waiters.push({ predicate, resolve })),
      8_000,
      `worker timed out; stderr=${stderr}`,
    );
  const ready = await waitFor(({ type }) => type === "ready");
  const startupMs = performance.now() - started;
  child.stdin.write(`${JSON.stringify({ type: "ping", requestId: "ping-1" })}\n`);
  await waitFor(({ type, requestId }) => type === "pong" && requestId === "ping-1");

  const secret = "must-not-cross-worker-protocol";
  child.stdin.write(
    `${JSON.stringify({ type: "ping", requestId: "secret", apiKey: secret })}\n`,
  );
  await waitFor(({ type, code }) => type === "error" && code === "invalid_command");
  child.stdin.write(`${JSON.stringify({ type: "run_fixture", requestId: "run-1" })}\n`);
  const result = await waitFor(
    ({ type, requestId }) => type === "result" && requestId === "run-1",
  );
  child.stdin.write(
    `${JSON.stringify({ type: "run_fixture", requestId: "cancel-1", delayMs: 1_000 })}\n`,
  );
  child.stdin.write(`${JSON.stringify({ type: "cancel", requestId: "cancel-1" })}\n`);
  const cancelled = await waitFor(
    ({ type, requestId }) => type === "result" && requestId === "cancel-1",
  );
  child.stdin.write(`${JSON.stringify({ type: "shutdown" })}\n`);
  const exit = await withTimeout(
    new Promise((resolve) => child.once("exit", (code, signal) => resolve({ code, signal }))),
    8_000,
    "worker did not shut down",
  );

  if (
    ready.runtime !== "single-pass" ||
    ready.environmentEvidencePresent !== true ||
    result.result?.outcome !== "accepted" ||
    cancelled.result?.outcome !== "cancelled" ||
    exit.code !== 0 ||
    stdout.includes(secret) ||
    stderr.includes(secret) ||
    stdout.includes(environmentSecret) ||
    stderr.includes(environmentSecret)
  ) {
    throw new Error(`worker protocol failed: ${JSON.stringify({ ready, result, cancelled, exit })}`);
  }
  return { startupMs, stdoutLines: messages.length, cleanExit: true };
}

async function crashIsolation() {
  const child = spawn(sidecar, [], { cwd: root, stdio: ["pipe", "pipe", "pipe"] });
  const lines = createInterface({ input: child.stdout });
  await withTimeout(
    new Promise((resolve) => lines.once("line", resolve)),
    8_000,
    "crash worker did not start",
  );
  child.stdin.write(`${JSON.stringify({ type: "crash" })}\n`);
  const code = await withTimeout(
    new Promise((resolve) => child.once("exit", resolve)),
    8_000,
    "crash worker did not terminate",
  );
  if (code !== 70) {
    throw new Error(`expected isolated crash exit 70, received ${code}`);
  }
  return { childExitCode: code, hostContinued: true };
}

function findFiles(path, predicate) {
  const found = [];
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const entryPath = join(path, entry.name);
    if (entry.isDirectory()) {
      found.push(...findFiles(entryPath, predicate));
    } else if (predicate(entryPath)) {
      found.push(entryPath);
    }
  }
  return found;
}

function directorySize(path) {
  return findFiles(path, () => true).reduce((total, file) => total + statSync(file).size, 0);
}

async function runTauriHost() {
  const bundleRoot = join(root, "src-tauri", "target", "debug", "bundle", "macos");
  const apps = readdirSync(bundleRoot)
    .filter((name) => name.endsWith(".app"))
    .map((name) => join(bundleRoot, name));
  if (apps.length !== 1) {
    throw new Error(`expected one app bundle, found ${apps.length}`);
  }
  const executables = findFiles(
    join(apps[0], "Contents", "MacOS"),
    (path) => path.endsWith("/cancan-document-normalizer-spike") && (statSync(path).mode & 0o111) !== 0,
  );
  if (executables.length !== 1) {
    throw new Error(`expected one app executable, found ${executables.length}`);
  }
  const child = spawn(executables[0], [], {
    cwd: root,
    env: { ...process.env, CANCAN_EVIDENCE_PROVIDER_KEY: "must-not-reach-tauri-sidecar" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let output = "";
  child.stdout.on("data", (chunk) => {
    output += chunk.toString();
  });
  child.stderr.on("data", (chunk) => {
    output += chunk.toString();
  });
  const code = await withTimeout(
    new Promise((resolve) => child.once("exit", resolve)),
    10_000,
    "Tauri host did not exit",
  );
  if (code !== 0 || !output.includes("tauri-sidecar-smoke-passed")) {
    throw new Error(`Tauri host smoke failed: code=${code} output=${output}`);
  }
  return { appBundleBytes: directorySize(apps[0]), hostExitCode: code };
}

const licenses = spawnSync("pnpm", ["licenses", "list", "--prod", "--json"], {
  cwd: root,
  encoding: "utf8",
});
if (licenses.status !== 0) {
  throw new Error(`license inventory failed: ${licenses.stderr}`);
}
mkdirSync(dist, { recursive: true });
writeFileSync(join(dist, "spike-dependency-licenses.json"), licenses.stdout);

const dev = await driveWorker(process.execPath, [join(dist, "worker.cjs")]);
const packagedRuns = [];
for (let attempt = 0; attempt < 3; attempt += 1) {
  packagedRuns.push(await driveWorker(sidecar, []));
}
const packagedStartupMs = packagedRuns
  .map(({ startupMs }) => Number(startupMs.toFixed(2)))
  .sort((left, right) => left - right);
const crash = await crashIsolation();
const tauri = await runTauriHost();
const require = createRequire(import.meta.url);
const { collectComparisonEvidence } = require(join(dist, "compare.cjs"));
const comparison = await collectComparisonEvidence();
const evidence = {
  environment: {
    node: process.version,
    platform: `${process.platform}-${process.arch}`,
    target: metadata.target,
  },
  dependencies: {
    aiSdk: "6.0.224",
    piAgentCore: "0.80.6",
    piAi: "0.80.6",
  },
  comparison,
  devWorker: { ...dev, startupMs: Number(dev.startupMs.toFixed(2)) },
  packagedWorker: {
    startupMsMedian: packagedStartupMs[1],
    startupMsRuns: packagedStartupMs,
    stdoutLinesPerRun: packagedRuns.map(({ stdoutLines }) => stdoutLines),
    cleanExit: packagedRuns.every(({ cleanExit }) => cleanExit),
  },
  crashIsolation: crash,
  sizes: {
    workerBundleBytes: metadata.workerBundleBytes,
    sidecarBytes: metadata.sidecarBytes,
    appBundleBytes: tauri.appBundleBytes,
  },
  signing: {
    sidecar: metadata.signing,
    notarization: "not tested; requires project signing identity and release slice",
  },
  spikeDependencyInventory: {
    bytes: Buffer.byteLength(licenses.stdout),
    sha256: createHash("sha256").update(licenses.stdout).digest("hex"),
  },
};
writeFileSync(join(dist, "evidence.json"), `${JSON.stringify(evidence, null, 2)}\n`);
process.stdout.write(`${JSON.stringify(evidence, null, 2)}\n`);
