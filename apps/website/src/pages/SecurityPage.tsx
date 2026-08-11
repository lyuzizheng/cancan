export function SecurityPage() {
  return (
    <>
      <div className="page-lede">
        <h1>Security model</h1>
        <p className="lede">
          CanCan handles financial evidence, so its security boundary is
          deliberate and narrow: an encrypted local Vault, OS-managed secrets,
          and no hosted surface.
        </p>
      </div>

      <h2 id="model">How CanCan protects your data</h2>
      <ul>
        <li>The Vault is a SQLCipher-encrypted database and encrypted file store on your Mac, unlocked by your password with an Argon2id-derived key.</li>
        <li>Vault keys, Touch ID-protected unlock, statement passwords, and OAuth refresh tokens live in the macOS Keychain.</li>
        <li>Document viewing renders in memory — no plaintext temp files. Source files are copied into the Vault with verified bytes and SHA-256 identity; duplicates and deleted files are handled deterministically.</li>
        <li>The renderer never receives raw file bytes, filesystem paths, hashes, locators, or database handles — only bounded, presentation-safe read models.</li>
        <li>Network access exists only for connections you enable: Gmail (official API, read-only scope), your own AI provider, and optional update checks that verify signed metadata.</li>
        <li>Releases are CI-built from immutable tags with checksums, signatures, and provenance — never developer-machine builds.</li>
      </ul>

      <h2 id="architecture">Local-first architecture</h2>
      <p>
        CanCan is a single-machine system: one Mac, one encrypted Vault, no
        hosted backend. The choices below are implemented in the open-source
        repository — <code>apps/desktop/src-tauri</code> — not promised on a
        slide.
      </p>
      <dl>
        <div className="meta-row">
          <dt>Database</dt>
          <dd>
            SQLCipher with vendored OpenSSL — the ledger is encrypted page by
            page, so no plaintext financial data ever reaches the disk.
          </dd>
        </div>
        <div className="meta-row">
          <dt>Documents</dt>
          <dd>
            Source files are sealed in XChaCha20-Poly1305 envelopes under a
            file-encryption subkey that HKDF-SHA256 separates from the master
            key, each envelope carrying its own random 24-byte nonce —
            authenticated encryption, so tampering is detected, not just
            discouraged.
          </dd>
        </div>
        <div className="meta-row">
          <dt>Key derivation</dt>
          <dd>
            Your password is stretched with Argon2id into a 256-bit wrapping
            key — the RFC 9106 low-memory profile (64 MiB, t=3, p=4) by
            default, or the OWASP minimum profile (19 MiB, t=2, p=1) when the
            first derivation exceeds the 750 ms unlock budget. That wrapping
            key seals a randomly generated master key; the password itself is
            never stored.
          </dd>
        </div>
        <div className="meta-row">
          <dt>Secrets</dt>
          <dd>
            Vault keys, Touch ID-protected unlock, OAuth refresh tokens, and statement
            passwords live in the macOS Keychain — never in the database or in
            plain files.
          </dd>
        </div>
        <div className="meta-row">
          <dt>Integrity</dt>
          <dd>
            Files are SHA-256 hashed on ingest and the digest is stored with
            the document; every decryption is authenticated by the envelope
            itself, and a Vault integrity sweep re-verifies stored bytes
            against the recorded digest — mismatches are marked missing, never
            served. Duplicates collapse to one verified copy.
          </dd>
        </div>
        <div className="meta-row">
          <dt>Process boundary</dt>
          <dd>
            The UI process receives bounded, presentation-safe read models
            only — raw file bytes, filesystem paths, hashes, locators, and
            database handles never cross into it.
          </dd>
        </div>
        <div className="meta-row">
          <dt>Network</dt>
          <dd>
            No listener, no telemetry, no analytics. Outbound connections exist
            only for capabilities you switch on: Gmail (official API,
            read-only scope), your own AI provider with a key you hold, and
            update checks that verify signed metadata before anything runs.
          </dd>
        </div>
        <div className="meta-row">
          <dt>Releases</dt>
          <dd>
            Artifacts are built by CI from immutable tags with checksums,
            signatures, and build provenance — never from a developer machine.
          </dd>
        </div>
      </dl>

      <h2 id="report">Reporting a vulnerability</h2>
      <p>
        Please report security issues privately — never in a public issue,
        discussion, or pull request.
      </p>
      <ul>
        <li>
          When the repository is public, use{" "}
          <a href="https://github.com/lyuzizheng/cancan/security/advisories">
            GitHub private vulnerability reporting
          </a>{" "}
          so we can coordinate a fix and disclosure.
        </li>
        <li>
          The address <code>security@cancan.money</code> is intended as the
          private contact and is being provisioned; until it is verified, the
          GitHub route (or direct contact with the maintainer) is authoritative.
        </li>
        <li>
          Do not include real financial data, account numbers, or secrets in a
          report — redacted reproduction steps are enough.
        </li>
      </ul>

      <h2 id="supported">Supported versions</h2>
      <p>
        CanCan is in a pre-1.0 preview line. Only the latest published{" "}
        <code>0.x Preview</code> release receives security fixes; fixes ship as
        new releases, never as silent patches. A security-critical update is
        explained prominently in the app and still waits for your approval to
        install.
      </p>
    </>
  );
}
