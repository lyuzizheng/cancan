type MockupVariant = "command" | "review" | "sources";

const LABELS: Record<MockupVariant, string> = {
  command: "Command Center — Money Overview",
  review: "Review queue — records waiting for your check",
  sources: "Sources — CanCan Inbox automation",
};

function Spine() {
  return (
    <div className="mock-spine" aria-hidden="true">
      <span className="mock-spine-logo" />
      <span className="mock-spine-item is-active" />
      <span className="mock-spine-item" />
      <span className="mock-spine-item" />
      <span className="mock-spine-item" />
      <span className="mock-spine-foot" />
    </div>
  );
}

function Row({ muted = false, wide = true }: { muted?: boolean; wide?: boolean }) {
  return (
    <span className={`mock-row${muted ? " is-muted" : ""}${wide ? "" : " is-narrow"}`} aria-hidden="true" />
  );
}

function CommandBody() {
  return (
    <div className="mock-body" aria-hidden="true">
      <span className="mock-heading" />
      <div className="mock-panel">
        <span className="mock-panel-title"><i className="point point-emerald" />Money Overview</span>
        <div className="mock-kv"><Row wide={false} /><span className="mock-value">12,456.78</span></div>
        <div className="mock-kv"><Row wide={false} /><span className="mock-value">980.50</span></div>
      </div>
      <div className="mock-panel">
        <span className="mock-panel-title"><i className="point point-emerald" />Recent activity</span>
        <Row /><Row muted /><Row muted />
      </div>
    </div>
  );
}

function ReviewBody() {
  return (
    <div className="mock-body" aria-hidden="true">
      <span className="mock-heading" />
      <div className="mock-panel">
        <span className="mock-panel-title"><i className="point point-amber" />Needs your check</span>
        <div className="mock-kv"><Row wide={false} /><span className="mock-chip">Review</span></div>
        <div className="mock-kv"><Row wide={false} /><span className="mock-chip">Review</span></div>
        <div className="mock-kv"><Row wide={false} /><span className="mock-chip is-quiet">Linked</span></div>
      </div>
      <Row muted /><Row muted />
    </div>
  );
}

function SourcesBody() {
  return (
    <div className="mock-body" aria-hidden="true">
      <span className="mock-heading" />
      <div className="mock-panel">
        <span className="mock-panel-title"><i className="point point-emerald" />CanCan Inbox is on</span>
        <Row /><Row muted />
        <span className="mock-chip">Check now</span>
      </div>
      <div className="mock-panel">
        <span className="mock-panel-title"><i className="point point-emerald" />Documents</span>
        <div className="mock-kv"><Row wide={false} /><span className="mock-value">Ready</span></div>
        <div className="mock-kv"><Row wide={false} /><span className="mock-value">Processing</span></div>
      </div>
    </div>
  );
}

const BODIES = {
  command: CommandBody,
  review: ReviewBody,
  sources: SourcesBody,
} as const;

/**
 * Schematic app preview drawn with tokens — a placeholder frame until real
 * product screenshots are approved. Honest wireframe, not a fake screenshot.
 */
export function Mockup({ variant }: { variant: MockupVariant }) {
  const Body = BODIES[variant];
  return (
    <figure className="mock-frame" role="img" aria-label={`Schematic preview: ${LABELS[variant]}`}>
      <div className="mock-chrome" aria-hidden="true">
        <span className="mock-chrome-dot" />
        <span className="mock-chrome-title">CanCan</span>
        <span className="mock-chrome-meta">schematic preview</span>
      </div>
      <div className="mock-window">
        <Spine />
        <Body />
      </div>
      <figcaption className="mock-caption">{LABELS[variant]}</figcaption>
    </figure>
  );
}
