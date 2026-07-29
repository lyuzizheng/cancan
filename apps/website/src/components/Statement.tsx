import type { CSSProperties } from "react";

/**
 * The process exhibit: a bank statement redrawn as a structured ledger table.
 * Deliberately a typeset document, not a fake app screenshot — honest
 * schematic, labeled as such, flat ink on paper.
 */
const ROWS = [
  { balance: "9,822.10", credit: "", date: "03/01", debit: "", item: "Balance b/f" },
  { balance: "16,022.10", credit: "6,200.00", date: "05/01", debit: "", item: "Salary — Acme Pte Ltd" },
  { balance: "15,967.90", credit: "", date: "09/01", debit: "54.20", item: "NTUC FairPrice" },
  { balance: "15,949.30", credit: "", date: "14/01", debit: "18.60", item: "Grab Transport" },
  { balance: "14,968.80", credit: "", date: "21/01", debit: "980.50", item: "DBS Card payment" },
  { balance: "14,972.22", credit: "3.42", date: "28/01", debit: "", item: "Interest" },
];

export function Statement() {
  return (
    <div className="sheet-cross">
      <div
        className="sheet"
        role="img"
        aria-label="Schematic example: a DBS bank statement structured into a ledger table with date, particulars, debit, credit, and running balance"
      >
        <div className="sheet-head mono-tag" aria-hidden="true">
          <span>Exhibit A — Statement, structured</span>
          <span>Schematic</span>
        </div>
        <p className="sheet-title" aria-hidden="true">
          Every record cites <em>its document.</em>
        </p>
        <table className="sheet-table" aria-hidden="true">
          <thead>
            <tr>
              <th>Date</th>
              <th>Particulars</th>
              <th className="num">Debit</th>
              <th className="num">Credit</th>
              <th className="num">Balance</th>
            </tr>
          </thead>
          <tbody>
            {ROWS.map((row, index) => (
              <tr className="sheet-row" key={row.item} style={{ "--d": `${620 + index * 90}ms` } as CSSProperties}>
                <td className="dim">{row.date}</td>
                <td className="item">{row.item}</td>
                <td className="num dim">{row.debit}</td>
                <td className="num">{row.credit}</td>
                <td className="num">{row.balance}</td>
              </tr>
            ))}
            <tr className="sheet-total sheet-row" style={{ "--d": "1220ms" } as CSSProperties}>
              <td className="dim" colSpan={4}>Closing balance · SGD</td>
              <td className="num sheet-balance" data-final="14972.22">14,972.22</td>
            </tr>
          </tbody>
        </table>
        <div className="sheet-foot mono-tag" aria-hidden="true">
          <span>Parser v0.4.2 · 14 records</span>
          <span>→ Review queue</span>
        </div>
      </div>
    </div>
  );
}
