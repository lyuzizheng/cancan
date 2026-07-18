import { chmodSync, copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

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
  entryPoints: [join(packageRoot, "src", "worker.ts")],
  outfile: join(dist, "worker.cjs"),
  bundle: true,
  format: "cjs",
  platform: "node",
  target: "node24",
  minify: true,
});

const seaConfig = join(dist, "sea-config.json");
const blob = join(dist, "sea-prep.blob");
writeFileSync(
  seaConfig,
  `${JSON.stringify({
    main: join(dist, "worker.cjs"),
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
const binary = join(binaries, `cancan-document-normalizer-${target}`);
copyFileSync(process.execPath, binary);
chmodSync(binary, 0o755);
run("codesign", ["--remove-signature", binary], { allowFailure: true, quiet: true });
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

const smoke = spawnSync(binary, [], {
  cwd: packageRoot,
  encoding: "utf8",
  env: {},
  input: `${JSON.stringify({
    type: "normalize",
    requestId: "build-smoke",
    documentId: "document-smoke",
    mimeType: "text/csv",
    content: [
      "CANCAN_SYNTHETIC_STATEMENT_V1",
      "provider=synthetic-bank",
      "statement_id=transfer-2026-07",
    ].join("\n"),
  })}\n{\"type\":\"shutdown\"}\n`,
});
if (smoke.status !== 0) {
  throw new Error(`sidecar smoke failed with status ${smoke.status}`);
}
const messages = smoke.stdout
  .trim()
  .split("\n")
  .map((line) => JSON.parse(line));
if (
  messages[0]?.type !== "ready" ||
  messages[0]?.protocolVersion !== 1 ||
  messages[0]?.runtime !== "single-pass-mock" ||
  messages[0]?.environmentCleared !== true ||
  messages[1]?.type !== "result" ||
  messages[1]?.requestId !== "build-smoke" ||
  messages[1]?.result?.status !== "classified"
) {
  throw new Error("sidecar smoke returned an invalid protocol transcript");
}
