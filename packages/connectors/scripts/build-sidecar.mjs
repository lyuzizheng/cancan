import { chmodSync, copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, spawnSync } from "node:child_process";

import { build } from "esbuild";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(packageRoot, "../..");
const dist = join(packageRoot, "dist");
const binaries = join(repoRoot, "apps", "desktop", "src-tauri", "binaries");

mkdirSync(dist, { recursive: true });
mkdirSync(binaries, { recursive: true });

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: packageRoot,
    encoding: "utf8",
    stdio: options.quiet ? "pipe" : "inherit",
  });
  if (result.status !== 0 && !options.allowFailure) {
    throw new Error(`${command} failed with status ${result.status}`);
  }
  return result;
}

await build({
  entryPoints: [join(packageRoot, "src", "gmail-oauth-worker-entry.ts")],
  outfile: join(dist, "gmail-oauth-worker.cjs"),
  bundle: true,
  format: "cjs",
  platform: "node",
  target: "node24",
  minify: true,
});

const seaConfig = join(dist, "gmail-oauth-sea-config.json");
const blob = join(dist, "gmail-oauth-sea-prep.blob");
writeFileSync(
  seaConfig,
  `${JSON.stringify({
    main: join(dist, "gmail-oauth-worker.cjs"),
    output: blob,
    disableExperimentalSEAWarning: true,
    useSnapshot: false,
    useCodeCache: false,
  })}\n`,
);
run(process.execPath, ["--experimental-sea-config", seaConfig]);

const target = run("rustc", ["--print", "host-tuple"], { quiet: true }).stdout.trim();
if (!target) {
  throw new Error("rustc did not return a target tuple");
}
const binary = join(binaries, `cancan-gmail-connector-${target}`);
copyFileSync(process.execPath, binary);
chmodSync(binary, 0o755);
run("codesign", ["--remove-signature", binary], {
  allowFailure: true,
  quiet: true,
});
run(process.execPath, [
  join(packageRoot, "node_modules", "postject", "dist", "cli.js"),
  binary,
  "NODE_SEA_BLOB",
  blob,
  "--sentinel-fuse",
  "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2",
  "--macho-segment-name",
  "NODE_SEA",
]);
run("codesign", ["--sign", "-", "--force", binary]);

const messages = await smokeSidecar(binary);
if (
  messages[0]?.type !== "ready" ||
  messages[0]?.protocolVersion !== 1 ||
  messages[0]?.runtime !== "gmail-oauth-v1" ||
  messages[0]?.environmentCleared !== true ||
  messages[1]?.type !== "error" ||
  messages[1]?.code !== "invalid_command"
) {
  throw new Error("Gmail connector sidecar smoke returned an invalid transcript");
}

function smokeSidecar(binary) {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, [], {
      cwd: packageRoot,
      env: {},
      stdio: ["pipe", "pipe", "pipe"],
    });
    let commandsSent = false;
    let stdout = "";
    const timeout = setTimeout(() => {
      child.kill();
      reject(
        new Error(
          "Gmail connector sidecar did not exit while parent stdin remained open",
        ),
      );
    }, 2_000);
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      if (!commandsSent && stdout.includes('"type":"ready"')) {
        commandsSent = true;
        child.stdin.write('{"type":"invalid"}\n{"type":"shutdown"}\n');
      }
    });
    child.on("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.on("close", (code) => {
      clearTimeout(timeout);
      if (code !== 0) {
        reject(
          new Error(`Gmail connector sidecar smoke failed with status ${code}`),
        );
        return;
      }
      try {
        resolve(
          stdout
            .trim()
            .split("\n")
            .map((line) => JSON.parse(line)),
        );
      } catch {
        reject(new Error("Gmail connector sidecar smoke returned invalid JSON"));
      }
    });
  });
}
