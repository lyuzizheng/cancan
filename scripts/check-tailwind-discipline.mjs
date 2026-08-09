/**
 * Tailwind/token discipline gate for the CanCan renderer.
 *
 * Rules (acceptance bar from the UI modernization program, specs 0008/0011):
 *   1. No Tailwind arbitrary-value syntax (`w-[317px]`, `text-[13px]`,
 *      `bg-[#…]`) — utilities must resolve to @theme tokens in
 *      packages/ui/src/tokens.css. Arbitrary *variant* selectors such as
 *      `data-[state=open]:…` are standard Tailwind and allowed.
 *   2. No hard-coded hex colors — color lives in tokens.css only.
 *   3. No `dark:` variant — the app is a per-region hybrid theme
 *      (Spine/Ledger token groups), not a light/dark toggle.
 *   4. No font-weight utilities above 560 (`font-semibold` and heavier) —
 *      documented financial hierarchy is expressed in tokens instead.
 *   5. No radius utilities larger than the md panel cap
 *      (`rounded-xl`/`2xl`/`3xl`; `rounded-full` → use `rounded-pill`).
 *      `rounded-lg` is the window-chassis token and stays allowed.
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const scanRoots = ["apps/desktop/src", "packages/ui/src"];
const extensions = new Set([".ts", ".tsx", ".css"]);

const rules = [
  {
    name: "arbitrary value — use @theme tokens",
    pattern: /[a-zA-Z@:]+-\[[^\]]+\](?!:)/,
  },
  {
    name: "hard-coded hex color — use tokens.css",
    pattern: /#[0-9a-fA-F]{3,8}\b/,
  },
  {
    name: "dark: variant — hybrid per-region theme instead",
    pattern: /(^|[\s"'`:])dark:/,
  },
  {
    name: "font weight above 560",
    pattern: /\bfont-(?:semi|extra)?bold\b/,
  },
  {
    name: "radius above the panel cap",
    pattern: /\brounded-(?:xl|2xl|3xl|full)\b/,
  },
];

function* walk(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      yield* walk(path);
    } else if (extensions.has(path.slice(path.lastIndexOf(".")))) {
      yield path;
    }
  }
}

let violations = 0;

for (const scanRoot of scanRoots) {
  for (const file of walk(join(root, scanRoot))) {
    const rel = relative(root, file);
    const lines = readFileSync(file, "utf8").split("\n");
    lines.forEach((line, index) => {
      for (const rule of rules) {
        if (rule.pattern.test(line)) {
          violations += 1;
          console.error(`${rel}:${index + 1}  ${rule.name}\n    ${line.trim()}`);
        }
      }
    });
  }
}

if (violations > 0) {
  console.error(`\ncheck-tailwind-discipline: ${violations} violation(s)`);
  process.exit(1);
}

console.log("check-tailwind-discipline: pass");
