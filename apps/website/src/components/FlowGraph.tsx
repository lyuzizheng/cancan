/**
 * The money-flow exhibit: sources on the left, one ledger on the right, and
 * rotating examples of the same dollar seen twice — a card repayment, a Wise
 * top-up chain, and a salary sweep — each linked into a single event. Wise is
 * drawn as a dashed goal channel, not coverage. Flat line art, animated
 * dashes, no glow. The link arc draws itself as it enters the viewport via a
 * pure-CSS scroll-driven animation (statically drawn where unsupported).
 */
const EXAMPLES = [
  {
    rows: [
      { amount: "−980.50", date: "21/01", item: "Card payment", source: "DBS Bank" },
      { amount: "+980.50", date: "21/01", item: "Payment received", source: "DBS Card" },
    ],
    tag: "[ Linked — one event ]",
  },
  {
    rows: [
      { amount: "−1,000.00", date: "03/02", item: "FAST — Wise top-up", source: "DBS Bank" },
      { amount: "+1,000.00", date: "03/02", item: "Top-up received", source: "Wise" },
      { amount: "−500.00", date: "04/02", item: "Transfer to A. Rahman", source: "Wise" },
    ],
    tag: "[ The goal — every hop counted once ]",
  },
  {
    rows: [
      { amount: "+6,400.00", date: "15/02", item: "Salary credit", source: "HSBC" },
      { amount: "−6,400.00", date: "16/02", item: "FAST — swept to DBS", source: "HSBC" },
      { amount: "+6,400.00", date: "16/02", item: "FAST received", source: "DBS Bank" },
    ],
    tag: "[ Linked — one event ]",
  },
];

export function FlowGraph() {
  return (
    <>
      <div
        className="flow-exhibit"
        role="img"
        aria-label="Schematic diagram: statements from DBS Bank, DBS Card, HSBC, and UOB flow into one CanCan ledger, with Wise shown as a dashed goal channel. Rotating examples link the same money recorded on two statements — a card repayment, a Wise top-up chain, and a salary sweep — into single events."
      >
      <svg className="flow-diagram" viewBox="0 0 760 430" aria-hidden="true">
        {/* base channels */}
        <path className="flow-edge" d="M 64 62 C 300 62, 400 208, 548 208" />
        <path className="flow-edge" d="M 64 132 C 280 132, 400 208, 548 208" />
        <path className="flow-edge" d="M 64 202 C 280 202, 400 208, 548 208" />
        <path className="flow-edge flow-goal" d="M 64 272 C 280 272, 400 208, 548 208" />
        <path className="flow-edge" d="M 64 342 C 300 342, 400 208, 548 208" />
        {/* packets — live channels only, not the goal */}
        <path className="flow-packet pkt-a" d="M 64 62 C 300 62, 400 208, 548 208" />
        <path className="flow-packet pkt-b" d="M 64 132 C 280 132, 400 208, 548 208" />
        <path className="flow-packet pkt-c" d="M 64 202 C 280 202, 400 208, 548 208" />
        <path className="flow-packet pkt-d" d="M 64 342 C 300 342, 400 208, 548 208" />
        {/* the link: same dollar, seen twice */}
        <path className="flow-link" d="M 52 62 C 16 78, 16 116, 52 132" />

        {/* source nodes */}
        <g className="flow-node">
          <rect x="52" y="56" width="12" height="12" />
          <text x="80" y="48">DBS BANK</text>
        </g>
        <g className="flow-node">
          <rect x="52" y="126" width="12" height="12" />
          <text x="80" y="118">DBS CARD</text>
        </g>
        <g className="flow-node">
          <rect x="52" y="196" width="12" height="12" />
          <text x="80" y="188">HSBC</text>
        </g>
        <g className="flow-node flow-node-goal">
          <rect x="52" y="266" width="12" height="12" />
          <text x="80" y="258">WISE</text>
          <text className="flow-soon" x="122" y="258">[ GOAL ]</text>
        </g>
        <g className="flow-node">
          <rect x="52" y="336" width="12" height="12" />
          <text x="80" y="328">UOB</text>
          <text className="flow-soon" x="116" y="328">[ COMING ]</text>
        </g>

        {/* ledger node */}
        <g className="flow-ledger">
          <rect x="548" y="196" width="24" height="24" />
          <text x="606" y="213">LEDGER</text>
          <text className="flow-ledger-sub" x="606" y="232">ONE EVENT PER DOLLAR</text>
        </g>
      </svg>

      <div className="pair-stage" aria-hidden="true">
        {EXAMPLES.map((example, index) => (
          <div className="pair-example" key={index}>
            <div className="pair-rows">
              {example.rows.map((row) => (
                <div className="pair-row" key={`${row.date}-${row.source}-${row.item}`}>
                  <span className="pair-date">{row.date}</span>
                  <span className="pair-source">{row.source}</span>
                  <span className="pair-item">{row.item}</span>
                  <span className="pair-amount">{row.amount}</span>
                </div>
              ))}
            </div>
            <div className="pair-link">
              <span className="pair-bracket" aria-hidden="true" />
              <span className="pair-tag">{example.tag}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
      <p className="pair-caption mono-tag">
        [ Wise appears as the goal, not as coverage — supported sources are listed below ]
      </p>
    </>
  );
}
