import type { CSSProperties } from "react";

import { ConsentArt, OverviewArt, ParseArt, ReconcileArt } from "../components/FeatureArt";
import { FlowGraph } from "../components/FlowGraph";
import { Wordmark } from "../components/Layout";
import { Statement } from "../components/Statement";
import { VaultSchematic } from "../components/VaultSchematic";

/* Fictional micro-ledger lines, current sources only — decorative texture. */
const DRIFT_ROWS = [
  "21/01 DBS BANK · CARD PAYMENT · −980.50 SGD · LINKED — 21/01 DBS CARD · PAYMENT RECEIVED · +980.50 · 20/01 HSBC · SALARY CREDIT · +6,400.00 SGD · LEDGERED · 19/01 DBS BANK · FAST TRANSFER · −120.00 SGD · REVIEWED · 18/01 DBS CARD · NTUC FAIRPRICE · −54.20 SGD · LEDGERED · ",
  "17/01 HSBC · GIRO — INSURANCE · −310.00 SGD · REVIEWED · 15/01 DBS BANK · GRAB TRANSPORT · −18.60 SGD · LEDGERED · 14/01 DBS CARD · PAYMENT RECEIVED · +500.00 · LINKED — 14/01 DBS BANK · CARD PAYMENT · −500.00 · 12/01 HSBC · INTEREST · +3.42 SGD · LEDGERED · ",
  "10/01 DBS BANK · SALARY CREDIT · +6,200.00 SGD · LEDGERED · 09/01 DBS CARD · COFFEE · −6.40 SGD · REVIEWED · 08/01 HSBC · STANDING ORDER · −850.00 SGD · LEDGERED · 05/01 DBS BANK · PAYNOW IN · +65.00 SGD · REVIEWED · 03/01 DBS CARD · PETROL · −72.10 SGD · LEDGERED · ",
  "02/01 HSBC · FAST TRANSFER · −200.00 SGD · REVIEWED · 31/12 DBS BANK · INTEREST · +3.42 SGD · LEDGERED · 30/12 DBS CARD · GROCERIES · −88.30 SGD · LEDGERED · 29/12 DBS BANK · CARD PAYMENT · −1,240.00 · LINKED — 29/12 DBS CARD · PAYMENT RECEIVED · +1,240.00 · ",
];

const DRIFT_STYLE = [
  { opacity: 0.1, top: "9%", "--dur": "88s" } as CSSProperties,
  { opacity: 0.07, top: "32%", "--dur": "112s" } as CSSProperties,
  { opacity: 0.07, top: "60%", "--dur": "96s" } as CSSProperties,
  { opacity: 0.08, top: "85%", "--dur": "124s" } as CSSProperties,
];

const TYPE_ROW_A = "DBS · POSB · UOB · OCBC · HSBC · Standard Chartered · Citi · Wise · GrabPay · CPF · Insurance · Brokerage · ";
const TYPE_ROW_B = "Brokerage · Insurance · CPF · GrabPay · Wise · Citi · Standard Chartered · HSBC · OCBC · UOB · POSB · DBS · ";

const FLOW_STEPS = [
  {
    body: "Add statement PDFs, CSVs, or images directly — or save them to your CanCan Inbox folder in iCloud Drive and CanCan picks up new files for you. No bank credentials, no screen scraping.",
    title: "Collect",
  },
  {
    body: "CanCan extracts evidence locally, then asks the AI provider you configure to propose structured records — with the exact parser and runtime versions recorded next to every run.",
    title: "Parse",
  },
  {
    body: "Records wait in Review. Check details, edit, and link the same money seen on two statements — a card payment leaving DBS, arriving at your card — into one event.",
    title: "Reconcile",
  },
  {
    body: "Accepted records become your ledger and Money Overview — every value traceable back to the document it came from, with typed Undo.",
    title: "Ledger",
  },
];

const FEATURES = [
  {
    body: "CanCan extracts evidence locally, then asks the AI provider you configure — with a key you hold — to propose structured records. The parser and runtime versions are recorded against every record, and the source document is filed in the encrypted Vault.",
    index: "04.1",
    title: <>AI-assisted parsing, <em>archived.</em></>,
  },
  {
    body: "Money moving between your own accounts appears on two statements — once out, once in. Link the pair during review and your own transfers stop posing as spending.",
    index: "04.2",
    title: <>Reconciled, <em>counted once.</em></>,
  },
  {
    body: "One allocation view across accounts and currencies, drawn only from approved ledger records — every figure traceable back to the page it came from.",
    index: "04.3",
    title: <>An overview <em>that cites its sources.</em></>,
  },
  {
    body: "Deeper AI analysis is being qualified before it ships. When it arrives: your provider, your key, your consent — each analysis individually approved. Skip AI entirely and nothing is sent anywhere.",
    index: "04.4",
    title: <>Analysis, <em>on your terms.</em></>,
  },
];

const FEATURE_ARTS = [<ParseArt />, <ReconcileArt />, <OverviewArt />, <ConsentArt />];

const VAULT_SPECS = [
  {
    detail: "SQLCipher ledger database and XChaCha20-Poly1305 document envelopes — authenticated encryption, plaintext never touches disk.",
    term: "At rest",
  },
  {
    detail: "Argon2id stretches your password into the Vault key; Vault keys, tokens, and statement passwords live in the macOS Keychain.",
    term: "Keys",
  },
  {
    detail: "Every document is SHA-256 hashed on the way in and re-verified on every read — tamper-evident, deduplicated, deterministic.",
    term: "Integrity",
  },
  {
    detail: "The interface receives presentation-safe read models only — no raw bytes, paths, hashes, or database handles cross into it.",
    term: "Isolation",
  },
  {
    detail: "Opt-in connections only: Gmail with read-only scope, your AI provider with a key you hold, update checks against signed metadata.",
    term: "Network",
  },
];

const CREED = [
  {
    note: "No sign-up, no sync service, nothing to breach on our side",
    word: "account",
  },
  {
    note: "Neither the app nor this site collects analytics or behavioral data",
    word: "analytics",
  },
  {
    note: "Gmail and AI go direct from your Mac, only with your consent",
    word: "silent network",
  },
];

const SOURCES = [
  { mark: "D", name: "DBS Bank", note: "Bank statements — parsed locally, review-first", status: "current", tag: "[ Current ]" },
  { mark: "D", name: "DBS Card", note: "Credit-card statements — parsed locally, review-first", status: "current", tag: "[ Current ]" },
  { mark: "H", name: "HSBC", note: "Bank statements — parsed locally, review-first", status: "current", tag: "[ Current ]" },
  { mark: "U", name: "UOB", note: "Targeted for the first public preview, gated on evidence", status: "coming", tag: "[ Coming ]" },
  { mark: "G", name: "Gmail", note: "Statement attachments — public after Google’s verification", status: "coming", tag: "[ Coming ]" },
];

const delay = (ms: number) => ({ "--d": `${ms}ms` }) as CSSProperties;

export function HomePage() {
  return (
    <>
      <section className="hero">
        <div className="hero-drift" aria-hidden="true">
          {DRIFT_ROWS.map((row, index) => (
            <div className="drift-row" key={index} style={DRIFT_STYLE[index]}>
              <div className="drift-track">
                <span>{row}</span>
                <span>{row}</span>
              </div>
            </div>
          ))}
        </div>
        <header className="hero-mast wrap">
          <Wordmark current />
          <a className="hero-mast-link mono-tag" href="https://github.com/lyuzizheng/cancan">
            GitHub ↗
          </a>
        </header>
        <div className="hero-core wrap">
          <h1>
            <span className="mask"><span style={delay(80)}>Every dollar</span></span>
            <span className="mask"><span style={delay(210)}>has a <em>source.</em></span></span>
          </h1>
          <p className="hero-sign hero-in" style={delay(360)}>
            CanCan — as in, all your money sources, <em>can.</em>
          </p>
          <p className="lede hero-in" style={delay(480)}>
            Bank here, a card there, a wallet for the weekends — your money is spread
            across more platforms than anyone can track by hand. CanCan collects the
            statements those platforms already issue, reads them on your Mac with an
            AI provider you configure, and reconciles them into one ledger where every
            number cites its document.
          </p>
          <div className="hero-actions hero-in" style={delay(580)}>
            <a className="button-solid" href="/download/">
              Get the preview<span className="button-arrow" aria-hidden="true">→</span>
            </a>
            <a className="button-line" href="/docs/">
              Read the docs
            </a>
          </div>
        </div>
        <div className="hero-foot wrap hero-in" style={delay(720)}>
          <div className="hero-meta mono-tag" aria-label="Product facts">
            <span>Local-first finance</span>
            <span>macOS 14 or later</span>
            <span>First preview in preparation</span>
            <span>Open source — Apache-2.0</span>
          </div>
          <span className="hero-cue mono-tag" aria-hidden="true">
            Scroll <span className="hero-cue-arrow">↓</span>
          </span>
        </div>
      </section>

      <section className="band">
        <div className="wrap">
          <div className="section">
            <div className="section-index">
              <span className="idx mono-tag">N°01</span>
              <span className="mono-tag">The reality</span>
            </div>
            <h2 className="section-title">
              <span className="mask"><span>Your money lives in <em>more places</em></span></span>
              <span className="mask"><span>than it did five years ago.</span></span>
            </h2>
            <div className="type-wall" aria-hidden="true">
              <div className="type-row type-row-a">
                <div className="type-track">
                  <span>{TYPE_ROW_A}</span>
                  <span>{TYPE_ROW_A}</span>
                </div>
              </div>
              <div className="type-row type-row-b">
                <div className="type-track">
                  <span>{TYPE_ROW_B}</span>
                  <span>{TYPE_ROW_B}</span>
                </div>
              </div>
            </div>
            <p className="sr-only">
              DBS, POSB, UOB, OCBC, HSBC, Standard Chartered, Citi, Wise, GrabPay,
              CPF, insurance, and brokerage — platforms a typical household juggles.
            </p>
            <p className="mono-tag specimen-caption">
              [ Platforms a typical household juggles — not a coverage claim ]
            </p>
            <p className="reality-pivot">
              Nobody keeps a spending diary — and nobody should have to.
              The statements already exist. <em>CanCan reads those.</em>
            </p>
          </div>
        </div>
      </section>

      <section className="band night">
        <div className="wrap">
          <div className="section">
            <div className="section-index">
              <span className="idx mono-tag">N°02</span>
              <span className="mono-tag">The link</span>
            </div>
            <h2 className="section-title">
              <span className="mask"><span>The same dollar, <em>seen twice.</em></span></span>
            </h2>
            <p className="lede">
              Money moves between your own accounts constantly — salary in, card paid off,
              savings swept aside. Each move appears on two statements, once as money out
              and once as money in. Untracked, it inflates your spending. CanCan links
              the pair into a single event.
            </p>
            <FlowGraph />
          </div>
        </div>
      </section>

      <section className="band">
        <div className="wrap">
          <div className="section">
            <div className="section-index">
              <span className="idx mono-tag">N°03</span>
              <span className="mono-tag">Process</span>
            </div>
            <h2 className="section-title">
              <span className="mask"><span>Four steps, <em>no surprises.</em></span></span>
            </h2>
            <div className="process-grid">
              <ol className="flow-list">
                {FLOW_STEPS.map((step, index) => (
                  <li className="flow-step" key={step.title}>
                    <span className="flow-num" aria-hidden="true">{index + 1}</span>
                    <div>
                      <h3>{step.title}</h3>
                      <p>{step.body}</p>
                    </div>
                  </li>
                ))}
              </ol>
              <div className="process-visual">
                <Statement />
              </div>
            </div>
          </div>
        </div>
      </section>

      <section className="band night">
        <div className="wrap">
          <div className="creed">
            <div className="section-index">
              <span className="idx mono-tag">Creed</span>
              <span className="mono-tag">What we refuse to build</span>
            </div>
            <div style={{ marginTop: "42px" }}>
              {CREED.map((line) => (
                <div className="creed-row" key={line.word}>
                  <h3>No <em>{line.word}.</em></h3>
                  <p>{line.note}</p>
                </div>
              ))}
            </div>
            <div className="vault-tech">
              <p className="mono-tag vault-tech-head">[ What "local" is built on ]</p>
              <h3 className="vault-tech-title">
                Your Mac is the <em>whole stack.</em>
              </h3>
              <VaultSchematic />
              <dl className="vault-specs">
                {VAULT_SPECS.map((spec) => (
                  <div key={spec.term}>
                    <dt>{spec.term}</dt>
                    <dd>{spec.detail}</dd>
                  </div>
                ))}
              </dl>
            </div>
            <p className="creed-links mono-tag">
              Engineering, not policy filler — read exactly how:{" "}
              <a href="/privacy/">Privacy</a> · <a href="/security/">Security</a>
            </p>
          </div>
        </div>
      </section>

      <section className="band">
        <div className="wrap">
          <div className="section">
            <div className="section-index">
              <span className="idx mono-tag">N°04</span>
              <span className="mono-tag">The standard</span>
            </div>
            <h2 className="section-title">
              <span className="mask"><span>Collected, reconciled,</span></span>
              <span className="mask"><span><em>accounted for.</em></span></span>
            </h2>
            <ul className="feature-list">
              {FEATURES.map((feature, index) => (
                <li className="feature-row" key={feature.index}>
                  <span className="feature-index">{feature.index}</span>
                  <h3>{feature.title}</h3>
                  <p>{feature.body}</p>
                  {FEATURE_ARTS[index]}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      <section className="band">
        <div className="wrap">
          <div className="section">
            <div className="section-index">
              <span className="idx mono-tag">N°05</span>
              <span className="mono-tag">Coverage</span>
            </div>
            <h2 className="section-title" id="sources-heading">
              <span className="mask"><span>Statement sources,</span></span>
              <span className="mask"><span><em>honestly</em> listed.</span></span>
            </h2>
            <div className="table-scroll">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Source</th>
                    <th>Status</th>
                    <th>Notes</th>
                  </tr>
                </thead>
                <tbody>
                  {SOURCES.map((source) => (
                    <tr key={source.name}>
                      <td className="source">
                        <span className="source-mark" aria-hidden="true">{source.mark}</span>
                        {source.name}
                      </td>
                      <td>
                        <span className={source.status === "current" ? "tag-current" : "tag-coming"}>
                          {source.tag}
                        </span>
                      </td>
                      <td className="note">{source.note}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="table-notes">
              A bank appears here only after a real parser profile exists for it — never on a
              wishlist. Automatic add stays off for every source until its confidence is
              separately qualified. Password-protected eStatements are a first-class case:
              unlock once, and the password stays in your macOS Keychain.
            </p>
            <p className="table-notes">
              Using POSB, OCBC, Standard Chartered, Citibank, or Wise?{" "}
              <a href="https://github.com/lyuzizheng/cancan/discussions">Tell us what you use</a>.
            </p>
          </div>
        </div>
      </section>

      <section className="band">
        <div className="wrap">
          <div className="section">
            <div className="section-index">
              <span className="idx mono-tag">N°06</span>
              <span className="mono-tag">Next</span>
            </div>
            <h2 className="section-title">
              <span className="mask"><span>The first preview,</span></span>
              <span className="mask"><span><em>honestly</em> scoped.</span></span>
            </h2>
            <div className="spec">
              <div className="spec-head mono-tag">
                <span>Spec — first public release</span>
                <span>0.x Preview</span>
              </div>
              <h3>A pre-1.0 preview, <em>not a stability promise.</em></h3>
              <p>
                It ships local file ingestion, the CanCan Inbox folder, statement parsing
                with review-first control, and source-backed money views.
              </p>
              <p>
                Planned and gated on their own evidence: Gmail connection for statement
                attachments and separately consented transaction-notification emails
                (public availability follows Google’s verification), and source coverage
                for DBS, HSBC, and UOB — some profiles start Review-only while their
                automatic-add confidence is calibrated against held-out evidence.
              </p>
              <p>
                Deeper AI analysis of your ledger is being qualified separately and ships
                only behind explicit consent — your provider, your key.
              </p>
              <p>
                Follow along on <a href="https://github.com/lyuzizheng/cancan">GitHub</a> —
                development, specs, and releases all happen in the open — or{" "}
                <a href="/docs/">read the docs</a>.
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className="band night">
        <div className="wrap">
          <div className="cta">
            <h2>
              <span className="mask"><span>Prove <em>it.</em></span></span>
            </h2>
            <p className="cta-sign">CanCan — as in, all your money sources, <em>can.</em></p>
            <p>GitHub Releases · Signed artifacts · No account</p>
            <div className="cta-actions">
              <a className="button-solid" href="/download/">
                Get the preview<span className="button-arrow" aria-hidden="true">→</span>
              </a>
              <a className="button-line" href="https://github.com/lyuzizheng/cancan">
                Star on GitHub
              </a>
            </div>
          </div>
        </div>
      </section>
    </>
  );
}
