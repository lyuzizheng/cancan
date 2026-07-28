/**
 * Deterministic gate for the CanCan public website. Runs against dist/ after
 * build and fails on broken internal links, non-HTTPS external references,
 * missing accessibility structure, or missing required disclosure content.
 * Dependency-free by design (docs/specs/0018 keeps the site static).
 */
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const dist = join(root, "dist");

/** Required phrases per page path (relative to dist/). */
const REQUIRED_CONTENT = {
  "index.html": [
    "local-first",
    "preview",
    "macOS",
  ],
  "download/index.html": [
    "GitHub Releases",
    "Apple Silicon",
    "SHA-256",
  ],
  "privacy/index.html": [
    "Google API Services User Data Policy",
    "Limited Use",
    "gmail.readonly",
    "attachment",
    "transaction-email",
    "Keychain",
  ],
  "security/index.html": [
    "vulnerability",
    "private",
    "supported",
  ],
  "docs/index.html": [
    "Vault",
    "Review",
  ],
};

const failures = [];

function fail(message) {
  failures.push(message);
}

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      out.push(...walk(path));
    } else {
      out.push(path);
    }
  }
  return out;
}

function attr(tag, name) {
  const match = new RegExp(`${name}\\s*=\\s*"([^"]*)"`, "i").exec(tag);
  return match ? match[1] : null;
}

function checkHtml(path) {
  const rel = path.slice(dist.length + 1);
  const html = readFileSync(path, "utf8");

  if (!/<html[^>]*\slang="[a-z-]+"/i.test(html)) {
    fail(`${rel}: <html> is missing a lang attribute`);
  }
  if (!/<title>[^<]{4,}<\/title>/i.test(html)) {
    fail(`${rel}: missing a real <title>`);
  }
  if (!/<meta[^>]*name="description"[^>]*content="[^"]{20,}"/i.test(html)) {
    fail(`${rel}: missing a meta description of at least 20 characters`);
  }
  const h1Count = (html.match(/<h1[\s>]/gi) ?? []).length;
  if (h1Count !== 1) {
    fail(`${rel}: expected exactly one <h1>, found ${h1Count}`);
  }
  for (const landmark of ["<main", "<nav", "<footer"]) {
    if (!html.includes(landmark)) {
      fail(`${rel}: missing ${landmark} landmark`);
    }
  }
  if (!/class="skip-link"/.test(html)) {
    fail(`${rel}: missing the skip link`);
  }
  if (/<script[\s>]/i.test(html)) {
    fail(`${rel}: runtime scripts are not allowed on the static public site`);
  }
  for (const phrase of [
    "folder is never modified",
    "parsing stays local and deterministic",
    "stale sources are marked",
  ]) {
    if (html.toLowerCase().includes(phrase)) {
      fail(`${rel}: contains superseded public claim "${phrase}"`);
    }
  }

  const svgTags = html.match(/<svg[^>]*>/gi) ?? [];
  for (const svg of svgTags) {
    if (!/aria-hidden="true"/.test(svg) && !/role="img"/.test(svg)) {
      fail(`${rel}: inline <svg> needs aria-hidden="true" or role="img" with a label`);
    }
  }

  const refTags = html.match(/<(?:a|link|img|script|source)[^>]*(?:href|src)=[^>]*>/gi) ?? [];
  const ids = new Set(
    (html.match(/\sid="[^"]+"/gi) ?? []).map((match) => match.slice(5, -1)),
  );
  for (const tag of refTags) {
    const ref = attr(tag, "href") ?? attr(tag, "src");
    if (!ref || ref.startsWith("mailto:") || ref.startsWith("tel:") || ref.startsWith("data:")) {
      continue;
    }
    if (ref.startsWith("http://")) {
      fail(`${rel}: non-HTTPS reference ${ref}`);
      continue;
    }
    if (ref.startsWith("https://")) {
      continue;
    }
    const [targetPath, anchor] = ref.split("#", 2);
    const base = targetPath === ""
      ? path
      : targetPath.startsWith("/")
        ? join(dist, targetPath)
        : resolve(dirname(path), targetPath);
    const candidates = targetPath.endsWith("/") || targetPath === "" || !/\.[a-z0-9]+$/i.test(targetPath)
      ? [join(base, "index.html"), base]
      : [base];
    if (targetPath !== "" && !candidates.some((candidate) => existsSync(candidate))) {
      fail(`${rel}: unresolved internal reference ${ref}`);
      continue;
    }
    if (anchor) {
      const targetHtml = targetPath === ""
        ? html
        : existsSync(candidates[0])
          ? readFileSync(candidates[0], "utf8")
          : "";
      const targetIds = new Set(
        (targetHtml.match(/\sid="[^"]+"/gi) ?? []).map((match) => match.slice(5, -1)),
      );
      if (targetPath === "" ? !ids.has(anchor) : !targetIds.has(anchor)) {
        fail(`${rel}: anchor #${anchor} has no target in ${targetPath || rel}`);
      }
    }
  }

  for (const phrase of REQUIRED_CONTENT[rel] ?? []) {
    if (!html.toLowerCase().includes(phrase.toLowerCase())) {
      fail(`${rel}: missing required content "${phrase}"`);
    }
  }
  if (rel === "privacy/index.html") {
    for (const phrase of [
      "Structured statement parsing requires an AI provider",
      "AI-dependent parsing waits",
      "only evidence matching an enabled rule may travel",
    ]) {
      if (!html.toLowerCase().includes(phrase.toLowerCase())) {
        fail(`${rel}: missing AI data-path disclosure "${phrase}"`);
      }
    }
  }
}

if (!existsSync(dist)) {
  fail("dist/ is missing; run pnpm --filter @cancan/website build first");
} else {
  const htmlFiles = walk(dist).filter((path) => path.endsWith(".html"));
  if (htmlFiles.length === 0) {
    fail("dist/ contains no HTML pages");
  }
  for (const page of Object.keys(REQUIRED_CONTENT)) {
    if (!existsSync(join(dist, page))) {
      fail(`required page ${page} is missing`);
    }
  }
  for (const path of htmlFiles) {
    checkHtml(path);
  }

  const cssFiles = walk(dist).filter((path) => path.endsWith(".css"));
  for (const css of cssFiles) {
    const text = readFileSync(css, "utf8");
    for (const match of text.matchAll(/url\("?([^")]+)"?\)/g)) {
      const ref = match[1];
      if (ref.startsWith("data:")) {
        continue;
      }
      if (ref.startsWith("http://")) {
        fail(`${css.slice(dist.length + 1)}: non-HTTPS css url ${ref}`);
        continue;
      }
      if (ref.startsWith("https://")) {
        fail(`${css.slice(dist.length + 1)}: remote css url ${ref} — assets must be self-hosted`);
        continue;
      }
      if (!existsSync(ref.startsWith("/") ? join(dist, ref) : resolve(dirname(css), ref))) {
        fail(`${css.slice(dist.length + 1)}: unresolved css url ${ref}`);
      }
    }
    if (/@font-face/.test(text) && !/font-display:\s*swap/.test(text)) {
      fail(`${css.slice(dist.length + 1)}: @font-face without font-display: swap`);
    }
  }
}

if (failures.length > 0) {
  console.error(`website check failed (${failures.length}):`);
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}
console.log("website check: links, accessibility structure, and disclosures pass");
