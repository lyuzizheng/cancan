import { createHash } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

import { build } from "esbuild";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dist = join(root, "dist");
const binaries = join(root, "src-tauri", "binaries");
mkdirSync(dist, { recursive: true });
mkdirSync(binaries, { recursive: true });

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    stdio: options.quiet ? "pipe" : "inherit",
  });
  if (result.status !== 0 && !options.allowFailure) {
    throw new Error(`${command} failed with status ${result.status}`);
  }
  return result;
}

await build({
  entryPoints: [join(root, "src", "worker.ts")],
  outfile: join(dist, "worker.cjs"),
  bundle: true,
  format: "cjs",
  platform: "node",
  target: "node24",
  minify: true,
  sourcemap: false,
});

await build({
  entryPoints: [join(root, "src", "compare.ts")],
  outfile: join(dist, "compare.cjs"),
  bundle: true,
  format: "cjs",
  platform: "node",
  target: "node24",
  sourcemap: false,
});

const seaConfigPath = join(dist, "sea-config.json");
const blobPath = join(dist, "sea-prep.blob");
writeFileSync(
  seaConfigPath,
  `${JSON.stringify(
    {
      main: join(dist, "worker.cjs"),
      output: blobPath,
      disableExperimentalSEAWarning: true,
      useSnapshot: false,
      useCodeCache: false,
    },
    null,
    2,
  )}\n`,
);
run(process.execPath, ["--experimental-sea-config", seaConfigPath]);

const target = run("rustc", ["--print", "host-tuple"], { quiet: true }).stdout.trim();
if (!target) {
  throw new Error("rustc did not return a target tuple");
}
const binaryPath = join(binaries, `cancan-document-agent-${target}`);
copyFileSync(process.execPath, binaryPath);
chmodSync(binaryPath, 0o755);
run("codesign", ["--remove-signature", binaryPath], { allowFailure: true, quiet: true });
run(process.execPath, [
  join(root, "node_modules", "postject", "dist", "cli.js"),
  binaryPath,
  "NODE_SEA_BLOB",
  blobPath,
  "--sentinel-fuse",
  "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2",
  "--macho-segment-name",
  "NODE_SEA",
]);
run("codesign", ["--sign", "-", "--force", binaryPath]);
run("codesign", ["--verify", "--strict", "--verbose=2", binaryPath]);

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
writeFileSync(
  join(dist, "build-metadata.json"),
  `${JSON.stringify(
    {
      nodeVersion: process.version,
      target,
      workerBundleBytes: statSync(join(dist, "worker.cjs")).size,
      sidecarBytes: statSync(binaryPath).size,
      sidecarSha256: sha256(binaryPath),
      signing: "ad-hoc",
    },
    null,
    2,
  )}\n`,
);
