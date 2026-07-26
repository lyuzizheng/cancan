// Guardrail: keep source files small enough to review, merge, and reason about.
// docs/specs/0001-repo-structure.md owns the rationale and the limits.
import { execSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

const LIMITS = [
  { pattern: /\.(test|spec)\.(ts|tsx)$/, max: 1200 },
  { pattern: /(^|\/)tests\.rs$/, max: 2500 },
  { pattern: /\.(ts|tsx|rs)$/, max: 800 },
];

// Ratchet-only exemptions for files that predate the guardrail. Shrink the
// ceiling as the file shrinks; never raise it. The open decomposition decisions
// live in docs/alignment-temp/alignment-progress.md.
const EXEMPTIONS = new Map([
  ["apps/desktop/src/app.tsx", 1750],
  ["apps/desktop/src-tauri/src/database/mod.rs", 3550],
  ["apps/desktop/src-tauri/src/runtime/documents.rs", 1000],
  ["apps/desktop/src-tauri/src/runtime/review.rs", 950],
  ["apps/desktop/src-tauri/src/runtime/tests.rs", 3650],
  ["apps/desktop/src-tauri/src/source_observations.rs", 950],
  ["apps/desktop/src-tauri/src/vault.rs", 900],
  ["apps/desktop/src-tauri/src/viewer.rs", 1000],
]);

const files = execSync(
  "git ls-files --cached --others --exclude-standard 'apps/*.ts' 'apps/*.tsx' 'apps/*.rs' 'packages/*.ts' 'packages/*.tsx'",
  { encoding: "utf8" },
)
  .split("\n")
  .filter(Boolean)
  .filter((file) => !file.endsWith(".d.ts"))
  .filter((file) => existsSync(file));

const failures = [];
for (const file of files) {
  const limit = LIMITS.find(({ pattern }) => pattern.test(file));
  if (!limit) {
    continue;
  }
  const max = Math.max(limit.max, EXEMPTIONS.get(file) ?? 0);
  const lines = readFileSync(file, "utf8").split("\n").length;
  if (lines > max) {
    failures.push(`${file}: ${lines} lines (max ${max})`);
  }
}

if (failures.length > 0) {
  console.error("Source files exceed the size guardrail (split them per docs/specs/0001-repo-structure.md first):");
  for (const failure of failures) {
    console.error(`  ${failure}`);
  }
  process.exit(1);
}

console.log(`File size guardrail passed for ${files.length} files.`);
