export function DownloadPage() {
  return (
    <>
      <div className="page-lede">
        <h1>Download CanCan</h1>
        <p className="lede">
          CanCan for macOS is distributed exclusively through GitHub Releases —
          the same signed artifacts serve direct download, the in-app updater,
          and the optional Homebrew Cask.
        </p>
      </div>

      <div className="pending-note">
        <strong>The first signed preview release is being prepared.</strong> No
        public artifacts exist yet; this page will link the verified release as
        soon as it is published. Watch the repository to be notified.
      </div>

      <h2 id="get-it">Get it</h2>
      <dl>
        <div className="meta-row">
          <dt>Releases</dt>
          <dd><a href="https://github.com/lyuzizheng/cancan/releases">github.com/lyuzizheng/cancan/releases</a></dd>
        </div>
        <div className="meta-row">
          <dt>Platform</dt>
          <dd>macOS 14 Sonoma or later on Apple Silicon</dd>
        </div>
        <div className="meta-row">
          <dt>Intel Macs</dt>
          <dd>The x86_64 build is under qualification and is not advertised yet — support is announced only after the full gate suite passes on real Intel hardware.</dd>
        </div>
        <div className="meta-row">
          <dt>Version line</dt>
          <dd>One <code>0.x Preview</code> channel — no alpha/beta tracks, honest preview copy</dd>
        </div>
      </dl>

      <h2 id="verify">Every artifact is verifiable</h2>
      <ul>
        <li>Each release traces to an immutable source tag and its GitHub Actions run.</li>
        <li>Installers ship with SHA-256 checksums, cryptographic signatures, and dependency/license (SBOM-style) reports.</li>
        <li>The in-app updater verifies signed update metadata before installing anything. Update checks are off by default; a manual check is always available.</li>
        <li>Homebrew, when available, pins a concrete version and checksum of the same GitHub Release artifact — never a separate build.</li>
      </ul>

      <h2 id="source">Prefer building from source?</h2>
      <p>
        CanCan is fully open source under Apache-2.0. Clone
        the <a href="https://github.com/lyuzizheng/cancan">repository</a> and run
        <code> ./scripts/setup-dev.sh</code> — one command installs the pinned
        toolchain and verifies the workspace.
      </p>
    </>
  );
}
