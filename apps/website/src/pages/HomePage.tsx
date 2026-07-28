import { Mockup } from "../components/Mockup";
import { Reveal } from "../components/Reveal";

const FLOW_STEPS = [
  {
    body: "Add statement PDFs, CSVs, or images directly — or save them to your CanCan Inbox folder in iCloud Drive and CanCan picks up new files for you.",
    title: "Collect",
  },
  {
    body: "Statements are read locally into structured records, with the exact parser version recorded next to every run.",
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
              CanCan turns bank statements into a ledger you control — parsed locally,
              reviewed by you, and traceable back to the exact document every record
              came from. No hosted account. No upload. No analytics.
            </p>
            <div className="hero-actions">
              <a className="button-primary" href="/download/">Get the preview</a>
              <a className="button-quiet" href="/docs/">Read the docs</a>
            </div>
            <p className="status-line">
              First public preview in preparation · macOS 14 or later · Open source, Apache-2.0
            </p>
          </div>
          <div className="hero-stage">
            <Mockup variant="command" />
          </div>
        </div>
      </section>

      <Reveal>
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
      </Reveal>

      <Reveal>
        <section className="feature" aria-labelledby="inbox-heading">
          <div className="feature-copy">
            <h2 id="inbox-heading">Add statements without opening the app</h2>
            <p>
              Save a statement to the <code>Cancan/Inbox</code> folder in iCloud Drive and it
              is waiting in CanCan the next time you open the app. CanCan only ever looks at
              the Inbox child of that folder — your folder stays outside the encrypted Vault,
              follows iCloud Drive’s privacy, and is never modified.
            </p>
            <p>
              Prefer manual control? Add files directly, drag them in, or use Open With.
              The Inbox is one intake channel, never a requirement.
            </p>
          </div>
          <Mockup variant="sources" />
        </section>
      </Reveal>

      <Reveal>
        <section className="feature is-flipped" aria-labelledby="review-heading">
          <div className="feature-copy">
            <h2 id="review-heading">Review first. Always.</h2>
            <p>
              Parsed records never jump straight into your books. A single Review queue lets
              you check amounts and dates, edit what the parser missed, and link related
              records — like a card repayment that appears on two statements — before adding
              them as one event.
            </p>
            <p>
              Every add is explicit, every undo is typed, and nothing rewrites committed
              history silently.
            </p>
          </div>
          <Mockup variant="review" />
        </section>
      </Reveal>

      <Reveal>
        <section className="feature" aria-labelledby="overview-heading">
          <div className="feature-copy">
            <h2 id="overview-heading">A money picture that cites its sources</h2>
            <p>
              The Money Overview shows balances per account and currency, derived from your
              own evidence. Freshness is visible, stale sources are marked, and there is no
              aggregator account in the middle holding your data.
            </p>
          </div>
          <Mockup variant="command" />
        </section>
      </Reveal>

      <Reveal>
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
      </Reveal>

      <Reveal>
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
      </Reveal>
    </>
  );
}
