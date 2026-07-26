export interface Notice {
  body: string;
  tone: "success" | "attention";
  title: string;
}

export function Feedback({
  action,
  body,
  title,
  tone,
}: Omit<Notice, "tone"> & { action?: () => void; tone: Notice["tone"] | "error" }) {
  return (
    <section className={`feedback feedback-${tone}`} aria-live="polite">
      <div><p className="feedback-title">{title}</p><p>{body}</p></div>
      {action ? <button className="button button-quiet" onClick={action} type="button">Try again</button> : null}
    </section>
  );
}
