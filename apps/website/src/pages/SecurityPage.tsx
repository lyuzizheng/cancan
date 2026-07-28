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
        <li>Vault keys, remembered unlock, statement passwords, and OAuth refresh tokens live in the macOS Keychain.</li>
        <li>Document viewing renders in memory — no plaintext temp files. Source files are copied into the Vault with verified bytes and SHA-256 identity; duplicates and deleted files are handled deterministically.</li>
        <li>The renderer never receives raw file bytes, filesystem paths, hashes, locators, or database handles — only bounded, presentation-safe read models.</li>
        <li>Network access exists only for connections you enable: Gmail (official API, read-only scope), your own AI provider, and optional update checks that verify signed metadata.</li>
        <li>Releases are CI-built from immutable tags with checksums, signatures, and provenance — never developer-machine builds.</li>
      </ul>

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
