# 0003. Gmail Collector Spec

## Goal

Collect supported statement attachments and transaction-notification evidence from one or more user-authorized Gmail mailboxes while preserving local-first behavior.

## Implementation blocker

Public OAuth verification, provider retention, Limited Use compatibility, and final one-time per-mailbox Gmail-to-AI disclosure wording remain open in the [active alignment register](../alignment-temp/alignment-progress.md). The mailbox-level authorization and payload boundary below are accepted, but do not claim public Gmail availability or submit verification until the real multi-mailbox data flow and public identity are reviewable.

## MVP decision

Use official Gmail API with Desktop OAuth Authorization Code Flow + PKCE + loopback redirect.

Gmail is one optional evidence channel, not the product's primary ingestion architecture. Manual add, drag/drop, Open With, and a user-selected inbox folder must remain useful without Gmail.

Do not use AI computer-use/browser automation as the primary Gmail architecture. Computer-use may remain a future fallback/experiment for websites or bank portals that do not expose usable APIs, but Gmail MVP should use the official API.

## Local-first OAuth architecture

CanCan should not proxy Gmail data through a CanCan server.

Flow:

```text
1. User starts Gmail connect flow in CanCan desktop app.
2. App generates PKCE code_verifier and code_challenge.
3. App starts a temporary local loopback listener on http://127.0.0.1:{random_port}/callback.
4. App opens the system browser to Google OAuth authorization URL.
5. User approves requested Gmail scope.
6. Google redirects authorization code to local loopback callback.
7. App exchanges code + code_verifier + client_id + redirect_uri directly with Google token endpoint.
8. App receives access_token and refresh_token.
9. App stores refresh_token in macOS Keychain / OS secret storage.
10. App stores access_token in memory or Keychain and refreshes as needed.
11. App calls Gmail API directly from local app.
12. App records one connected-mailbox identity, stores its refresh token under a mailbox-scoped Keychain key, and may repeat this flow to add another mailbox.
13. Email metadata, attachments, cache, index, and sync state are stored locally in encrypted vault/database.
```

Desktop apps cannot safely keep a client secret. PKCE is required to protect the authorization code exchange.

## Google Cloud setup

Use separate Google Cloud projects and credentials for development/testing and public production. Normal users authorize their own Gmail account through the project-owned production Desktop OAuth client; they should not need to create a Google Cloud project or paste OAuth credentials.

For development/testing, Google Cloud needs:

```text
Gmail API enabled
OAuth consent screen configured
OAuth client application type: Desktop app
redirect URI using loopback pattern
explicit test users while the consent screen remains in testing
```

The production Desktop OAuth `client_id` is a public identifier and may be distributed in the app's build configuration. A Desktop app cannot keep a `client_secret`; do not rely on one. Developer-only client-ID override may exist outside the committed repository for local integration tests. OAuth tokens and any mistakenly issued client secret must never enter source control, fixtures, logs, or release artifacts.

The public production project needs its own project-owned Desktop OAuth client and completed verification before Gmail is advertised as generally available.

## Scope policy

Use minimum Gmail scope.

Required scope:

```text
https://www.googleapis.com/auth/gmail.readonly
```

Google classifies `gmail.readonly` as a Restricted scope. Request no broader scope and keep the implementation/data-use justification aligned with read-only statement discovery and import.

Allowed actions:

```text
search/list messages
read metadata
read message body when needed
read attachment metadata
download attachments
read history for incremental sync
```

Disallowed actions:

```text
send email
modify labels
mark read/unread
delete email
change mailbox settings
```

## Public OAuth verification track

Treat verification as a release workstream, not a last-minute console toggle. Before submission:

```text
choose the public app identity, support contact, and authorized domain
prove authorized-domain ownership in Google Search Console
publish a public homepage and privacy policy on that domain
explain local storage, optional AI transfer, retention, deletion, and Google API Services User Data Policy Limited Use compliance
prepare a complete OAuth demo video and per-scope justification
verify that the consent-screen copy exactly matches the running app and website
submit brand and restricted-scope verification, then leave schedule margin for review questions
```

Google states that verification can take several weeks. A security assessment may also be required depending on whether Restricted-scope data is stored on or transmitted through servers or third-party services. MVP has no CanCan-hosted backend, but optional BYO-AI transfer is part of the real data flow and must be disclosed; Google makes the final verification and assessment determination. Any future hosted AI relay requires a renewed Google-policy, privacy, and security-assessment review before it can receive Gmail-derived data.

Authoritative references:

- [Gmail API scopes](https://developers.google.com/workspace/gmail/api/auth/scopes)
- [OAuth for native apps](https://developers.google.com/identity/protocols/oauth2/native-app)
- [Restricted-scope verification](https://developers.google.com/identity/protocols/oauth2/production-readiness/restricted-scope-verification)
- [Google Workspace API user-data policy](https://developers.google.com/workspace/workspace-api-user-data-developer-policy)

## UX model

Use guided rules plus expert query editing.

Top section: guided builder.

```text
Mode: statement attachment | transaction notification | CanCan Inbox attachment
Provider: DBS | HSBC | UOB | another supported source | detect automatically
Document type: bank statement | credit card statement | export | transaction notification
Required keywords
Excluded keywords
Sender/from hint
Has attachment toggle where applicable
Filename hints
Start date
Overlap days
Enabled toggle
Auto-scan toggle
```

Bottom section: expert Gmail query preview/editor.

The user can manually edit the generated query. Preserve both:

```text
generated_query
manual_query_override
active_query
```

The generic CanCan Inbox attachment rule is narrow by default. Before enabling it, the user chooses either a dedicated recipient/plus-alias address or an existing Gmail label; the generated query requires that boundary plus a supported attachment type and excludes Spam/Trash. Do not silently default to every self-sent or every PDF/CSV attachment. Broader expert queries require an explicit warning and `Test rule` preview before activation.

## Provider policy

Gmail rules should use fixed supported provider hints. Users should not create arbitrary provider integrations in MVP.

Manual source/support requests should be tracked as future feature requests, not custom live integrations.

Each provider-specific Gmail rule normally points to a user-configured Money Source. A generic `CanCan Inbox attachment` rule may leave the source unset so trusted classification can route supported evidence to one of the user's configured Money Sources. CanCan may provide useful defaults, and the user may edit or override the query.

Each rule also belongs to exactly one connected Gmail mailbox. One Money Source may have different rules across multiple mailboxes, and one mailbox may serve multiple Money Sources. Adding a mailbox repeats the official Google OAuth flow and one truthful mailbox-level AI-processing disclosure; refresh tokens, authorization state, reconnect state, rule cursors, and failures remain mailbox-scoped. Connecting the same mailbox address twice must reconnect the existing mailbox instead of creating duplicate sync authority.

Disconnecting a mailbox disables all of its rules, cancels queued sync jobs, stops an in-flight sync at its next safe boundary before another remote fetch/capture, and deletes its refresh token from Keychain. The mailbox identity, disabled rules, cursors, prior captured evidence, message mappings, and evidence tombstones remain. Re-authorizing the same normalized mailbox address reconnects that existing identity and resumes from the retained rule/cursor state rather than creating duplicate authority or re-importing prior messages.

A Gmail rule may have provider and source hints, but downloaded documents and message evidence still go through classifier/parser verification. Do not trust the rule or source assignment alone.

Transaction-notification rules additionally define exact supported sender/domain and authentication fingerprints. Before body normalization, the connector rejects Spam/Trash and deterministically verifies Gmail-provided authentication results against the provider package's aligned domain policy. Missing, failed, or mismatched authentication remains untrusted `Needs attention` evidence and cannot auto-link or auto-commit. A matching display-name or `From` header alone is never sufficient.

## Data model

```text
gmail_accounts
- id
- mailbox_address
- secret_storage_key
- connection_status
- ai_processing_authorized_at
- created_at
- updated_at
```

`mailbox_address` is the normalized address returned by Gmail `users.getProfile` under the authorized read-only scope and is unique within the Vault. It is safe display metadata and reconnect identity, but Gmail message idempotency still uses the internal `gmail_account_id` plus message ID. The refresh token lives in Keychain under `secret_storage_key`, never SQLite.

```text
gmail_search_rules
- id
- gmail_account_id
- money_source_id nullable only for generic inbox attachment rules
- name
- evidence_mode = attachment | transaction_notification
- provider_hint
- document_type_hint
- required_keywords_json
- excluded_keywords_json
- sender_hint
- generated_query
- manual_query_override
- active_query
- start_after
- overlap_days
- auto_scan_enabled
- enabled
- created_at
- updated_at
```

```text
gmail_sync_states
- id
- rule_id
- last_success_at
- last_scan_started_at
- last_scan_finished_at
- latest_history_id
- cursor_json
- last_error_json
- created_at
- updated_at
```

```text
statement_secret_refs
- id
- money_source_id unique
- secret_storage_key
- status
- hint_label
- created_at
- updated_at
```

`statement_secret_refs.money_source_id` is unique, so it stores at most one reference and status per Money Source. Actual statement passwords live in macOS Keychain/OS secret storage, never SQLite.

`transaction_notification` rules are provider-specific and require a configured Money Source because their body selectors and interpretation are provider-owned.

## Storage policy

Use minimum storage.

Attachment default:

```text
store message id
store thread id if useful
store sender/from
store subject
store date/internalDate
store the bounded label and sender-authentication validation summary needed to explain trust
store attachment metadata
store downloaded attachment bytes in file vault
store file hash and source document metadata
```

Do not store full email body by default unless needed for evidence/parser behavior or explicitly enabled.

Connecting one mailbox requires one explicit mailbox-level authorization that accurately describes Gmail read access, the configured AI provider, and the allowed attachment/body processing. After that authorization, CanCan may read messages needed by any enabled rule for that mailbox and may send the complete matched attachment or provider-approved body required by the selected parser to the configured AI provider without asking again per rule. The product still executes user-configured queries instead of bulk-ingesting the mailbox, and AI payloads never include OAuth tokens, statement passwords, unrelated messages, or messages outside an enabled rule. Revoking or disconnecting the mailbox removes this live authority; already captured local evidence follows the normal retained-evidence lifecycle.

### Send to yourself

A user may share a bank PDF/CSV from a phone to their own Gmail address. A generic CanCan Inbox rule can find that attachment, capture it, and let trusted classification select the configured Money Source and account. CanCan does not provide a hosted upload email address in MVP: that would add server custody, retention, abuse handling, and a second security boundary.

### Transaction notifications without attachments

A supported provider's transaction email may contain one useful record and no file. This is allowed only through an explicit provider-owned `transaction_notification` rule under a mailbox whose one-time authorization disclosed body capture and configured-AI processing.

The connector creates one deterministic canonical message-evidence envelope containing the authenticated mailbox/message ID, internal date, sender, subject, the provider-selected body fields or bounded body text, and attachment metadata. The encrypted envelope becomes a normal source-document artifact and the provider parser normalizes it into at most the supported records. The app does not manufacture a fake PDF or insert a transaction directly from email fields.

Transaction-notification evidence is provisional unless the provider package proves a posted status. It can appear as `Pending from email`, suggest a relationship, and later link to a posted statement record, but it cannot satisfy monthly statement coverage or bypass snapshot reconciliation and commit policy.

Provider classification and sender authentication are separate gates: a message that looks like DBS content but fails the DBS sender/domain policy does not run the trusted DBS notification parser.

## Protected PDF statements

Some statement PDFs are password protected.

Flow:

```text
Attachment downloaded
-> file hash dedupe
-> detect password-protected PDF
-> source_document.document_status = locked
-> create review/job prompt: password needed
-> user enters password
-> app tests unlock locally
-> user may use it once or update the saved password for this Money Source
-> extraction resumes
```

Rules:

- password entry is optional;
- password unlock happens locally;
- passwords must not be sent to AI providers;
- passwords must not be written to logs, parse payloads, raw_json, normalized_json, or backups by default;
- one saved password per Money Source is referenced by `statement_secret_refs` and stored in OS secret storage;
- Gmail and manual imports assigned to the same Money Source reuse that password;
- if it fails, prompt for `Use once` or `Update saved password`; MVP stores no password history or unlocked duplicate PDF;
- users can delete saved statement passwords from Settings.

## Sync strategy

Initial login/full sync:

```text
messages.list with q query
messages.get for matching message details
attachments.get for matching attachments
store latest usable historyId when available
```

Incremental sync:

```text
prefer history.list from saved historyId where possible
fall back to query-based overlap window when history is expired/unavailable
```

Gmail push notification uses Google Cloud Pub/Sub and a backend webhook, which does not fit the fully local-first MVP. Prefer local polling:

```text
app startup
app wake/resume
manual refresh
configurable interval while app is running
```

Auto-scan should be configurable and can be disabled. Manual scan must always be available.

Gmail sync execution is globally serial within one unlocked Vault: at most one `gmail_sync_rule` job runs at a time across every connected mailbox and rule. A failed or retry-delayed rule releases the worker so later queued rules can run; it does not create mailbox-level parallelism or monopolize the queue.

Overlapping rules and overlap-window retries use `gmail_account_id` plus Gmail-message ID for connector idempotency. The mapping remains attached to a deleted source-document tombstone, so an unchanged Gmail message is not silently restored on the next scan or after an envelope-format update. Only an explicit user restore action can re-enable it. Attachment SHA-256 and canonical-envelope SHA-256 remain artifact identity; financial record identity remains owned by the parser contract.

## Test rule UX

Before enabling a rule, provide `Test rule`:

```text
show recent matching messages
show attachment names and mime types
show provider/document classification hints
show whether each attachment would import, skip, unlock, or dedupe
```

## Error handling

Gmail auth/sync errors should appear in multiple places:

```text
Command Center source status
Jobs page with technical details
Gmail rule settings with reconnect CTA
```

Do not only show a transient toast.

Protected PDF errors should appear as locked document jobs with a clear unlock action.

## Tests

Use mocked Gmail API fixtures for:

```text
OAuth callback success
OAuth callback failure
refresh token flow
add two independently authorized mailboxes
same mailbox address reconnect converges instead of duplicating the mailbox
disconnect disables rules, cancels queued work, stops before another remote fetch/capture, and deletes the token while retaining cursors/evidence
reconnect resumes the retained mailbox/rule identity without duplicate imports
rules for one Money Source running against different mailboxes
multiple mailbox/rule sync jobs execute globally one at a time
first scan from start date
incremental scan with historyId
historyId expired fallback to overlap query
duplicate attachment hash
same attachment from same email
rule disabled
auto-scan disabled
OAuth/token error
restricted/insufficient scope error
no matching emails
PDF attachment import
password-protected PDF attachment import
CSV attachment import
message body not stored by default
transaction notification body requires an explicit rule under an authorized mailbox
Gmail attachment/body cloud-AI transfer is impossible before the one-time mailbox authorization and remains scoped to enabled rules
generic inbox rule requires a dedicated recipient/alias or label and excludes unrelated attachments
Spam/Trash and spoofed/misaligned provider sender fixtures never become trusted notification records
transaction notification creates canonical encrypted message evidence, not a direct ledger write
the same Gmail message reached by overlapping rules is idempotent
deleted Gmail evidence remains suppressed across later scans until explicit restore
send-to-self attachment routes through trusted source/account classification
email notification later matched to the posted statement row without duplicate ledger impact
```

## Acceptance criteria

- Gmail connect uses Desktop OAuth + PKCE + loopback redirect.
- A Vault may connect multiple Gmail mailboxes through repeated official OAuth flows; tokens, rules, cursors, reconnect state, and failures remain mailbox-scoped.
- One truthful authorization per mailbox covers Gmail read access and configured-AI processing for all enabled rules under that mailbox; CanCan does not prompt again for each rule.
- Disconnect disables that mailbox's rules, cancels/stops sync work before further capture, deletes its refresh token, and retains mailbox/rule/cursor/evidence identities for convergent reconnect.
- No Gmail data touches a CanCan server.
- Refresh token is stored in local secret storage.
- Only read-only Gmail scope is requested.
- User can create guided rules and edit expert query.
- Provider-specific rules belong to a configured Money Source; only the generic CanCan Inbox attachment rule may defer assignment to trusted classification.
- One Money Source may own different search rules across multiple connected mailboxes; every captured item still queues the shared `parse_document` path rather than a source-specific parser job type.
- Generic Inbox discovery starts from a dedicated recipient/alias or user-selected label; broad mailbox attachment collection is never the silent default.
- User can test a rule before enabling it.
- App supports startup/wake/manual/configurable polling sync.
- Re-running sync does not duplicate already imported attachments.
- Recurring sync never restores user-deleted evidence automatically.
- Supported no-attachment transaction emails require an explicit provider-owned rule under a mailbox whose authorization disclosed body capture, become encrypted canonical evidence, and remain provisional until posted evidence and policy gates resolve them.
- Gmail-derived attachments or provider-approved bodies reach a configured cloud AI provider only after the one-time mailbox authorization; tokens, passwords, unrelated messages, and out-of-rule data never do.
- Transaction-notification parsing requires provider-owned sender/domain authentication checks and excludes Spam/Trash; display names and content resemblance alone are untrusted.
- CanCan provides no hosted inbound email address in MVP; send-to-self ingestion uses the user's authorized Gmail mailbox.
- Password-protected PDFs can be detected, unlocked locally, and optionally tied to a saved secret reference.
- Errors are visible and actionable.
- Development/test credentials and test users are isolated from the production OAuth project.
- Public release does not advertise Gmail connection until the production consent screen, website disclosures, restricted-scope justification, and required Google verification are complete.
