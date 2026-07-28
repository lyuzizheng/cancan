/**
 * Pre-renders every route into a real static HTML file after `vite build`
 * (client) and `vite build --ssr` have run. The site stays fully readable
 * without JavaScript; hydration only adds progressive enhancement.
 */
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const dist = join(root, "dist");
const ssrEntry = join(root, "dist-ssr", "entry-server.js");

const ROUTES = [
  {
    description:
      "CanCan is a local-first personal finance workspace for macOS: statements stay encrypted on your Mac and every record traces back to its source document.",
    out: "index.html",
    route: "home",
    title: "CanCan — Local-first financial evidence vault",
  },
  {
    description:
      "Download CanCan for macOS from verified GitHub Releases: signed artifacts, checksums, signed updater metadata, and an honest pre-1.0 preview channel.",
    out: "download/index.html",
    route: "download",
    title: "Download CanCan — verified GitHub Releases",
  },
  {
    description:
      "How CanCan handles your data: encrypted local Vault, Keychain secrets, no analytics, Gmail read-only scope with separate attachment and transaction-email consents, and Google Limited Use compliance.",
    out: "privacy/index.html",
    route: "privacy",
    title: "Privacy at CanCan — local-first by design",
  },
  {
    description:
      "CanCan's security model, private vulnerability-reporting route, and supported-version policy for the pre-1.0 preview line.",
    out: "security/index.html",
    route: "security",
    title: "CanCan security model and vulnerability reporting",
  },
  {
    description:
      "Documentation for CanCan's shipping capabilities: Vault, evidence intake, CanCan Inbox, Money Sources, Review, and Money Overview.",
    out: "docs/index.html",
    route: "docs",
    title: "CanCan documentation",
  },
  {
    description: "The requested page does not exist on the CanCan website.",
    out: "404.html",
    route: "not-found",
    title: "Page not found — CanCan",
  },
];

const { render } = await import(pathToFileURL(ssrEntry).href);
const template = readFileSync(join(dist, "index.html"), "utf8");

for (const { description, out, route, title } of ROUTES) {
  const head = [
    `<title>${title}</title>`,
    `<meta name="description" content="${description}">`,
  ].join("\n  ");
  const html = template
    .replace("<!--app-head-->", head)
    .replace("<!--app-html-->", render(route));
  const target = join(dist, out);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, html);
}

rmSync(join(root, "dist-ssr"), { force: true, recursive: true });
console.log(`website prerender: ${ROUTES.length} routes written to dist/`);
