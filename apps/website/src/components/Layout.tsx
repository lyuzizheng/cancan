import type { ReactNode } from "react";

export type Route = "home" | "download" | "privacy" | "security" | "docs" | "not-found";

export function routeFromPath(pathname: string): Route {
  const path = pathname.replace(/\/+$/, "") || "/";
  switch (path) {
    case "/":
      return "home";
    case "/download":
      return "download";
    case "/privacy":
      return "privacy";
    case "/security":
      return "security";
    case "/docs":
      return "docs";
    default:
      return "not-found";
  }
}

const NAV_ITEMS: { href: string; label: string; route: Route }[] = [
  { href: "/", label: "Overview", route: "home" },
  { href: "/download/", label: "Download", route: "download" },
  { href: "/privacy/", label: "Privacy", route: "privacy" },
  { href: "/security/", label: "Security", route: "security" },
  { href: "/docs/", label: "Docs", route: "docs" },
];

export function Wordmark({ current = false }: { current?: boolean }) {
  return (
    <a className="wordmark" href="/" aria-current={current ? "page" : undefined}>
      <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
        <rect x="1.5" y="1.5" width="21" height="21" rx="5" stroke="#E9EEE9" strokeOpacity="0.5" />
        <circle cx="8" cy="12" r="2.2" fill="#2F7D55" />
        <path d="M13 8.5h6M13 12h6M13 15.5h4" stroke="#E9EEE9" strokeOpacity="0.85" strokeWidth="1.6" strokeLinecap="round" />
      </svg>
      CanCan
    </a>
  );
}

export function Layout({ route, children }: { route: Route; children: ReactNode }) {
  return (
    <>
      <a className="skip-link" href="#main">Skip to content</a>
      <header className="site-header">
        <div className="site-header-inner">
          <Wordmark current={route === "home"} />
          <nav className="site-nav" aria-label="Site">
            {NAV_ITEMS.map((item) => (
              <a
                aria-current={item.route === route ? "page" : undefined}
                href={item.href}
                key={item.route}
              >
                {item.label}
              </a>
            ))}
            <a href="https://github.com/lyuzizheng/cancan">GitHub</a>
          </nav>
        </div>
      </header>
      <main className="page-main" id="main">{children}</main>
      <footer className="site-footer">
        <div className="site-footer-inner">
          <p>CanCan — local-first financial evidence vault. Open source under Apache-2.0.</p>
          <nav aria-label="Footer">
            <a href="https://github.com/lyuzizheng/cancan">GitHub</a>
            <a href="https://github.com/lyuzizheng/cancan/discussions">Discussions</a>
            <a href="https://github.com/lyuzizheng/cancan/issues">Issues</a>
            <a href="/privacy/">Privacy</a>
            <a href="/security/">Security</a>
          </nav>
        </div>
      </footer>
    </>
  );
}
