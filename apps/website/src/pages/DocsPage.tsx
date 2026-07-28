export function DocsPage() {
  return (
    <>
      <div className="page-lede">
        <h1>CanCan documentation</h1>
        <p className="lede">
          Help for the current pre-release build and the first-preview behavior
          still being completed. Each section distinguishes what exists now from
          what remains pending.
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
        <li><strong>CanCan Inbox</strong>: choose your Cancan folder in iCloud Drive once. CanCan creates or reuses only its Inbox and Backups children, reads statements only from Inbox, and never modifies, moves, or deletes the source files you place there.</li>
        <li>Documents show truthful states — Ready, Processing, Needs attention, File deleted — so you always know what is happening.</li>
      </ul>

      <h2 id="statement-sources">Supported statement sources</h2>
      <ul>
        <li><strong>DBS bank statements</strong> and <strong>DBS credit-card statements</strong> — supported in the current build; every parsed record waits in Review before it can be added.</li>
        <li><strong>HSBC bank statements</strong> — the same review-first handling.</li>
        <li><strong>UOB statements</strong> — targeted for the first public preview; no parser exists yet, so they are not listed as supported.</li>
        <li><strong>Password-protected eStatements</strong> from any source — CanCan unlocks them locally, and you may save one password per Money Source in your macOS Keychain.</li>
        <li>Bank not listed? Request it on <a href="https://github.com/lyuzizheng/cancan/discussions">GitHub Discussions</a>. A source is listed only after a real parser profile exists for it — newer sources may stay Review-only while their automatic-add confidence is calibrated.</li>
      </ul>
      <p>
        In the first public preview, CanCan performs capture, encryption,
        extraction, validation, and ledger writes locally. Structured parsing
        uses the AI provider you configure directly from the local app. Live
        provider setup is not exposed in the current pre-release build yet;
        capture and document viewing continue while AI-dependent parsing waits.
      </p>

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
