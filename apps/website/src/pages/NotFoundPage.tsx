export function NotFoundPage() {
  return (
    <div className="page-lede">
      <h1>Page not found</h1>
      <p className="lede">
        This page isn’t in the ledger. Head back to the{" "}
        <a href="/">overview</a> or the <a href="/docs/">documentation</a>.
      </p>
    </div>
  );
}
