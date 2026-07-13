export function AppShell() {
  return (
    <main className="foundation-shell">
      <section className="foundation-card" aria-labelledby="app-title">
        <p className="foundation-kicker">Local-first foundation</p>
        <h1 id="app-title">CanCan</h1>
        <p className="foundation-copy">
          The production workspace is ready for the first source-backed flow.
        </p>
        <dl className="foundation-status" aria-label="Foundation status">
          <div>
            <dt>Runtime</dt>
            <dd>Desktop shell ready</dd>
          </div>
          <div>
            <dt>Network</dt>
            <dd>No capabilities enabled</dd>
          </div>
          <div>
            <dt>Data</dt>
            <dd>No vault opened</dd>
          </div>
        </dl>
      </section>
    </main>
  );
}
