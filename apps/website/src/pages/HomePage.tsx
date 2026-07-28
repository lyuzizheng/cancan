import { Mockup } from "../components/Mockup";

const FLOW_STEPS = [
  {
    body: "Add statement PDFs, CSVs, or images directly — or save them to your CanCan Inbox folder in iCloud Drive and CanCan picks up new files for you.",
    title: "Collect",
  },
  {
    body: "For the first public preview, CanCan extracts evidence locally, then asks the AI provider you configure to propose structured records. The exact parser and runtime versions stay recorded with every run.",
    title: "Parse",
  },
  {
    body: "Records wait in one Review queue. Check details, edit, link related records such as card repayments, then add them together.",
    title: "Review",
  },
  {
    body: "Accepted records become your ledger and Money Overview — every value traceable back to the document it came from, with typed Undo.",
    title: "Ledger",
  },
];

export function HomePage() {
  return (
    <>
      <section className="hero">
        <div className="hero-inner">
          <div className="hero-copy">
            <p className="hero-eyebrow">Local-first finance · macOS</p>
            <h1>Your financial evidence, kept on your Mac.</h1>
            <p className="lede">
              CanCan turns bank statements into a ledger you control — processed by
              the local app, reviewed by you, and traceable back to the exact document
              every record came from. No hosted account. Nothing leaves your Mac by
              default. No analytics.
            </p>
            <div className="hero-actions">
              <a className="button-primary" href="/download/">Get the preview</a>
              <a className="button-quiet" href="/docs/">Read the docs</a>
            </div>
            <p className="status-line">
              <span className="status-item">First public preview in preparation ·</span>{" "}
              <span className="status-item">macOS 14 or later ·</span>{" "}
              <span className="status-item">Open source, Apache-2.0</span>
            </p>
          </div>
          <div className="hero-stage">
            <Mockup variant="command" />
          </div>
        </div>
      </section>

      <section className="flow" aria-labelledby="flow-heading">
        <h2 id="flow-heading">From statement to ledger in four calm steps</h2>
        <ol className="flow-steps">
          {FLOW_STEPS.map((step, index) => (
            <li className="flow-step" key={step.title}>
              <span className="flow-index" aria-hidden="true">{`0${index + 1}`}</span>
              <h3>{step.title}</h3>
              <p>{step.body}</p>
            </li>
          ))}
        </ol>
      </section>

      <section className="sources" aria-labelledby="sources-heading">
          <h2 id="sources-heading">Statement sources, honestly listed</h2>
          <ul className="source-groups">
            <li className="source-group">
              <h3><span className="point point-emerald" aria-hidden="true" />In the current build — review-first</h3>
              <ul className="source-list">
                <li>DBS bank statements</li>
                <li>DBS credit-card statements</li>
                <li>HSBC bank statements</li>
              </ul>
              <p>
                In current builds, every parsed record waits for your review —
                nothing enters your ledger automatically. Password-protected
                eStatements are a first-class case: unlock once, and the password
                stays in your macOS Keychain.
              </p>
            </li>
            <li className="source-group">
              <h3><span className="point point-amber" aria-hidden="true" />Coming with the first preview</h3>
              <ul className="source-list">
                <li>UOB statements</li>
                <li>Gmail statement attachments — pending Google’s verification</li>
              </ul>
              <p>
                Gmail imports only attachments matching rules you enable, directly
                from your Mac, and becomes publicly available after Google verifies
                the app.
              </p>
            </li>
            <li className="source-group">
              <h3><span className="point" aria-hidden="true" />The listing rule</h3>
              <p>
                A bank appears here only after a real parser profile exists for
                it — never on a wishlist. Automatic add stays off for every
                source until its confidence is separately qualified. Coverage
                grows from what the project actually uses, then from your requests.
              </p>
              <p>
                Using POSB, OCBC, Standard Chartered, Citibank, or Wise?{" "}
                <a href="https://github.com/lyuzizheng/cancan/discussions">Tell us what you use</a>.
              </p>
            </li>
          </ul>
          <p className="sources-note">
            Multi-currency is first-class: each account keeps its own currency —
            SGD, USD, and more. No forced base currency, no exchange-rate guesses.
          </p>
      </section>

      <section className="feature" aria-labelledby="inbox-heading">
          <div className="feature-copy">
            <h2 id="inbox-heading">Add statements without opening the app</h2>
            <p>
              Save a statement to the <code>Cancan/Inbox</code> folder in iCloud Drive and it
              is waiting in CanCan the next time you open the app. CanCan only ever looks at
              the Inbox child of that folder. It creates or reuses only the <code>Inbox</code>{" "}
              and <code>Backups</code> children, and never modifies, moves, or deletes the
              statement files you place there.
            </p>
            <p>
              Prefer manual control? Add files directly in the app.
              The Inbox is one intake channel, never a requirement.
            </p>
          </div>
          <Mockup variant="sources" />
      </section>

      <section className="feature is-flipped" aria-labelledby="review-heading">
          <div className="feature-copy">
            <h2 id="review-heading">Review first. Auto-add must earn it.</h2>
            <p>
              In the current build, nothing enters your books automatically. A single
              Review queue lets you check amounts and dates, edit what the parser
              missed, and link related records — like a card repayment that appears
              on two statements — before adding them as one event.
            </p>
            <p>
              The preview adds automatic addition only for records that pass
              independently qualified confidence gates; everything else still waits
              for you. Every add is explicit, every undo is typed, and nothing
              rewrites committed history silently.
            </p>
          </div>
          <Mockup variant="review" />
      </section>

      <section className="feature" aria-labelledby="overview-heading">
          <div className="feature-copy">
            <h2 id="overview-heading">A money picture that cites its sources</h2>
            <p>
              The Money Overview shows balances per account and currency, derived from your
              own evidence, with the date behind each balance kept visible. There is no
              aggregator account in the middle holding your data.
            </p>
          </div>
          <Mockup variant="command" />
      </section>

      <section className="principles" aria-labelledby="principles-heading">
          <h2 id="principles-heading">Local-first, verifiably</h2>
          <ul className="principle-list">
            <li>
              <h3><span className="point point-emerald" aria-hidden="true" />Encrypted Vault</h3>
              <p>Documents and data live in a SQLCipher-encrypted Vault on your Mac, unlocked by your password. Secrets stay in the macOS Keychain.</p>
            </li>
            <li>
              <h3><span className="point point-emerald" aria-hidden="true" />No CanCan backend</h3>
              <p>There is no account system, no sync service, and no upload pipeline. Optional connections — Gmail, your own AI provider — go directly from your Mac, under your consent.</p>
            </li>
            <li>
              <h3><span className="point point-emerald" aria-hidden="true" />No telemetry</h3>
              <p>Neither the app nor this website collects analytics or behavioral data. Crash reporting is separate, opt-in, and off by default.</p>
            </li>
          </ul>
      </section>

      <section className="roadmap" aria-labelledby="preview-heading">
          <h2 id="preview-heading">The first preview, honestly scoped</h2>
          <p>
            The first public release is a pre-1.0 preview, not a stability promise. It ships
            local file ingestion, the CanCan Inbox folder, statement parsing with
            review-first control, and source-backed money views.
          </p>
          <p>
            Planned for the preview and gated on their own evidence: Gmail connection for
            statement attachments and separately consented transaction-notification emails
            (public availability follows Google’s verification), and source coverage for
            DBS, HSBC, and UOB — some profiles start Review-only while their automatic-add
            confidence is calibrated against held-out evidence.
          </p>
          <p>
            Follow along on <a href="https://github.com/lyuzizheng/cancan">GitHub</a> —
            development, specs, and releases all happen in the open.
          </p>
      </section>
    </>
  );
}
