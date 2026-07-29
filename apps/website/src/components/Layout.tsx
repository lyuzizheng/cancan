import type { ReactNode } from "react";

export type Route = "home" | "download" | "privacy" | "security" | "docs" | "not-found";

function Mark() {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="1" y="1" width="22" height="22" fill="none" stroke="#3ED67F" strokeOpacity="0.85" strokeWidth="1.4" />
      <rect x="5.4" y="5.4" width="6" height="6" fill="#3ED67F" />
      <path d="M14 7h5M14 12h5M14 17h3.2" stroke="#ECE9DF" strokeWidth="1.5" strokeLinecap="square" />
    </svg>
  );
}

export function Wordmark({ current = false }: { current?: boolean }) {
  return (
    <a className="wordmark" href="/" aria-current={current ? "page" : undefined}>
      <Mark />
      CanCan
    </a>
  );
}

export function Layout({ route, children }: { route: Route; children: ReactNode }) {
  return (
    <>
      <a className="skip-link" href="#main">Skip to content</a>
      {route !== "home" && (
        <div className="sub-back">
          <div className="wrap">
            <a className="mono-tag" href="/">← CanCan — Overview</a>
          </div>
        </div>
      )}
      <main className={`page-main ${route === "home" ? "page-home" : "page-sub"}`} id="main">{children}</main>
      {route !== "download" && (
        <a className="float-cta" href="/download/">
          Get preview<span className="button-arrow" aria-hidden="true">→</span>
        </a>
      )}
      <footer className="site-footer">
        <div className="wrap">
          <div className="footer-nav">
            <nav className="footer-col" aria-label="Product">
              <h4>Product</h4>
              <a href="/download/">Download</a>
              <a href="/docs/">Documentation</a>
            </nav>
            <nav className="footer-col" aria-label="Project">
              <h4>Project</h4>
              <a href="https://github.com/lyuzizheng/cancan">GitHub</a>
              <a href="https://github.com/lyuzizheng/cancan/discussions">Discussions</a>
              <a href="https://github.com/lyuzizheng/cancan/issues">Issues</a>
            </nav>
            <nav className="footer-col" aria-label="Trust">
              <h4>Trust</h4>
              <a href="/privacy/">Privacy</a>
              <a href="/security/">Security</a>
            </nav>
          </div>
          <p className="foot-giant" aria-hidden="true">CanCan</p>
          <div className="footer-legal">
            <span>Open source — Apache-2.0</span>
            <span>No analytics · No tracking · No hosted backend</span>
          </div>
        </div>
      </footer>
    </>
  );
}
