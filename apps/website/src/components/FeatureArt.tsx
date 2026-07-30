/**
 * Line-art exhibits for the feature ledger rows — one flat technical drawing
 * per claim, in the site's hairline idiom: currentColor strokes that invert
 * with the row hover, one green accent per drawing, and a scroll-driven
 * draw-in where supported (statically drawn otherwise).
 */

/** 04.1 — a statement scanned by the parser, then sealed into the archive. */
export function ParseArt() {
  return (
    <svg className="feat-art" viewBox="0 0 120 120" aria-hidden="true">
      <path className="art-line" pathLength={1} d="M30 14h32l18 18v52H30z" />
      <path className="art-line" pathLength={1} d="M62 14v18h18" />
      <path className="art-line art-faint" pathLength={1} d="M40 46h28M40 55h34M40 64h20" />
      <line className="art-beam" x1="22" y1="74" x2="88" y2="74" />
      <rect className="art-line art-accent" pathLength={1} x="60" y="86" width="36" height="20" />
      <path className="art-line art-accent" pathLength={1} d="M68 96h20" />
    </svg>
  );
}

/** 04.2 — the same dollar seen on two statements, linked into one event. */
export function ReconcileArt() {
  return (
    <svg className="feat-art" viewBox="0 0 120 120" aria-hidden="true">
      <rect className="art-line" pathLength={1} x="10" y="27" width="8" height="8" />
      <rect className="art-line" pathLength={1} x="10" y="81" width="8" height="8" />
      <path className="art-line" pathLength={1} d="M18 31c30 0 34 27 54 27" />
      <path className="art-line" pathLength={1} d="M18 85c30 0 34-27 54-27" />
      <rect className="art-line art-accent" pathLength={1} x="72" y="54" width="8" height="8" />
      <line className="art-line art-accent" pathLength={1} x1="80" y1="58" x2="100" y2="58" />
      <rect className="art-line" pathLength={1} x="100" y="54" width="8" height="8" />
      <text className="art-note" x="104" y="78" textAnchor="middle">×1</text>
    </svg>
  );
}

/** 04.3 — an overview whose bars trace back to the page they came from. */
export function OverviewArt() {
  return (
    <svg className="feat-art" viewBox="0 0 120 120" aria-hidden="true">
      <path className="art-bar" pathLength={1} d="M24 64V46" />
      <path className="art-bar" pathLength={1} d="M38 64V32" />
      <path className="art-bar" pathLength={1} d="M52 64V40" />
      <path className="art-bar" pathLength={1} d="M66 64V24" />
      <line className="art-line" pathLength={1} x1="16" y1="64" x2="74" y2="64" />
      <path className="art-cite" d="M66 24c26 4 32 28 24 52" />
      <rect className="art-line" pathLength={1} x="72" y="76" width="32" height="28" />
      <path className="art-line art-faint" pathLength={1} d="M78 85h20M78 92h14" />
    </svg>
  );
}

/** 04.4 — your key, your provider, analysis behind a consent gate. */
export function ConsentArt() {
  return (
    <svg className="feat-art" viewBox="0 0 120 120" aria-hidden="true">
      <circle className="art-line" pathLength={1} cx="32" cy="58" r="13" />
      <circle className="art-line art-faint" pathLength={1} cx="32" cy="58" r="4" />
      <path className="art-line" pathLength={1} d="M45 58h38M72 58v11M82 58v8" />
      <rect className="art-gate" x="92" y="38" width="14" height="40" />
      <path className="art-line art-accent" pathLength={1} d="M99 52v8" />
    </svg>
  );
}
