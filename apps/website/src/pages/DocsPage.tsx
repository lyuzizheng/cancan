export function DocsPage() {
  return (
    <>
      <div className="page-lede">
        <h1>CanCan documentation</h1>
        <p className="lede">
          Help for the capabilities that actually ship. CanCan is in pre-1.0
          preview; behavior is documented honestly as it exists in the current
          build.
        </p>
      </div>

      <h2 id="vault">Your Vault</h2>
      <ul>
        <li><strong>Create</strong> a Vault with one password on first launch. It encrypts your documents and data on this Mac.</li>
        <li><strong>Unlock</strong> with that password each session, or let CanCan remember the Vault on this Mac — the key stays in your macOS Keychain and you can forget it anytime.</li>
        <li><strong>Save your recovery file</strong> when prompted and store it privately. Anyone holding it can recover compatible Vault data.</li>
        <li><strong>Lock Vault</strong> from the header whenever you step away; all finance and review state clears immediately.</li>
      </ul>

      <h2 id="evidence">Adding evidence</h2>
      <ul>
        <li><strong>Add file</strong> in Sources to import statement PDFs, CSVs, PNGs, or JPEGs. CanCan saves a verified copy into your Vault before parsing.</li>
        <li><strong>CanCan Inbox</strong>: choose your Cancan folder in iCloud Drive once, then save statements to its Inbox child — CanCan checks on unlock, when files change, and when you ask. The folder is never modified, and the sibling Backups folder is never read for statements.</li>
        <li>Documents show truthful states — Ready, Processing, Needs attention, File deleted — so you always know what is happening.</li>
      </ul>

      <h2 id="sources">Money Sources</h2>
      <p>
        Documents are routed to Money Sources (your bank, card, or wallet).
        Select a source to see only its documents. When CanCan finds accounts it
        has not seen before, it asks you to confirm them before their records
        can be added.
      </p>

      <h2 id="review">Review and your ledger</h2>
      <ul>
        <li>Parsed records wait in the <strong>Review</strong> queue — nothing enters your ledger automatically in the current build.</li>
        <li>Open an item to check details, <strong>edit</strong> amount/date, or <strong>remove</strong> it. If something changed underneath, CanCan reloads the latest version and tells you.</li>
        <li>Related records — like a card repayment appearing on two statements — are <strong>linked</strong> and added together as one event.</li>
        <li>Select items and <strong>add</strong> them in one batch; each group reports its outcome.</li>
        <li>Recent activity supports <strong>Undo</strong> for eligible steps, with the reversal recorded explicitly.</li>
      </ul>

      <h2 id="overview">Money Overview</h2>
      <p>
        The Overview shows assets and liabilities per account and currency,
        derived from your evidence, plus recent activity and anything that needs
        your attention. Amounts are exact values from your documents — no
        rounding through floats, no external feed.
      </p>

      <h2 id="help">Getting help</h2>
      <ul>
        <li>Questions and how-tos: <a href="https://github.com/lyuzizheng/cancan/discussions">GitHub Discussions</a> (Questions is the first stop).</li>
        <li>Reproducible bugs: <a href="https://github.com/lyuzizheng/cancan/issues">Issues</a> — please use the bug form and never include financial data or secrets.</li>
        <li>Security issues: see the <a href="/security/">security page</a> — always privately.</li>
      </ul>
    </>
  );
}
