export function PrivacyPage() {
  return (
    <>
      <div className="page-lede">
        <h1>Privacy at CanCan</h1>
        <p className="lede">
          CanCan is local-first: your documents, ledger, and secrets stay on your
          Mac. This page describes how the app and this website handle data,
          including the Google API Services User Data Policy disclosures required
          for Gmail connection.
        </p>
      </div>

      <h2 id="local-first">Your data stays on your Mac</h2>
      <ul>
        <li>Documents and financial data live in a SQLCipher-encrypted Vault on your Mac, unlocked only by your password.</li>
        <li>Secrets — Vault keys, statement passwords, OAuth tokens — are stored in the macOS Keychain, never in plain files.</li>
        <li>Statements are parsed locally. Operational logs are local-only, redacted, retained for 30 days, and exported only by you.</li>
        <li>CanCan has no hosted backend, no account system, and no sync service. There is nothing to breach on our side because nothing leaves your Mac by default.</li>
        <li>Neither the app nor this website collects analytics, behavioral telemetry, or tracking data. Crash reporting is separate, opt-in, and off by default.</li>
      </ul>

      <h2 id="gmail">Gmail connection — two separate consents</h2>
      <p>
        Gmail is an optional evidence channel, planned for the first public
        preview and available only after Google’s required verification
        completes. Authorization happens between your Mac and Google directly:
        the OAuth flow (PKCE, loopback redirect) runs in the local app, tokens
        are stored in your macOS Keychain, and mailbox data is kept in your
        encrypted Vault. No Gmail data passes through a CanCan server — none exists.
      </p>
      <p>
        CanCan requests only <code>gmail.readonly</code>, a Google Restricted
        scope. It can search, read, and download matching messages and
        attachments. It cannot send email, modify labels, mark messages read or
        unread, delete email, or change mailbox settings.
      </p>
      <p>Each connected mailbox carries two independent switches:</p>
      <ul>
        <li>
          <strong>Attachment ingestion</strong> — statement attachments (PDF,
          CSV, images) matching your rules are downloaded into your Vault as
          evidence. This works with no AI provider configured at all.
        </li>
        <li>
          <strong>Transaction-email body ingestion</strong> — a separately
          consented capability. Only for provider-approved senders you enable,
          the text of transaction-notification emails is parsed into records.
          This consent is bound to your current AI provider and its disclosure;
          changing provider or disclosure turns transfer off until you
          re-consent per mailbox — Gmail itself stays connected.
        </li>
      </ul>
      <p>
        You can connect multiple mailboxes; tokens, rules, cursors, and failures
        stay isolated per mailbox. Disconnecting a mailbox deletes its refresh
        token from the Keychain and stops its rules; previously captured
        evidence remains in your Vault under your control.
      </p>
      <p className="section-note">
        Limited Use disclosure: CanCan’s use and transfer to any other app of
        information received from Google APIs will adhere to the{" "}
        <a href="https://developers.google.com/terms/api-services-user-data-policy">
          Google API Services User Data Policy
        </a>
        , including the Limited Use requirements.
      </p>

      <h2 id="ai">Optional AI processing</h2>
      <p>
        Statement parsing can use an AI provider you configure yourself
        (bring-your-own-key). Only evidence selected by an enabled rule travels,
        directly from your Mac to that provider, under the mailbox-level consent
        described above. Your provider keys live only in your local OS secret
        store. With AI setup skipped, nothing is sent anywhere — attachments
        still arrive, and parsing stays local and deterministic.
      </p>

      <h2 id="contact">Contact</h2>
      <p>
        The public addresses <code>support@cancan.money</code> and{" "}
        <code>security@cancan.money</code> are intended and will be verified
        before the preview ships; they are not yet provisioned. Until then,
        reach the project on{" "}
        <a href="https://github.com/lyuzizheng/cancan/discussions">GitHub Discussions</a>{" "}
        for questions and see the <a href="/security/">security page</a> for
        vulnerability reporting.
      </p>
    </>
  );
}
