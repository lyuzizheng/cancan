# 0003. Gmail Collector Spec

## Goal

Automatically collect financial statement evidence from Gmail while preserving local-first behavior.

## Implementation blocker

Public OAuth verification and Gmail-data transfer to cloud AI remain open in the [active alignment register](../alignment-temp/alignment-progress.md). Do not claim public Gmail availability, freeze the AI disclosure, or submit verification until the real data flow and public identity are reviewable.

## MVP decision

Use official Gmail API with Desktop OAuth Authorization Code Flow + PKCE + loopback redirect.

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
12. Email metadata, attachments, cache, index, and sync state are stored locally in encrypted vault/database.
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
Provider: DBS | UOB | Wise | supported source
Document type: bank statement | credit card statement | export | supported type
Required keywords
Excluded keywords
Sender/from hint
Has attachment toggle
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

## Provider policy

Gmail rules should use fixed supported provider hints. Users should not create arbitrary provider integrations in MVP.

Manual source/support requests should be tracked as future feature requests, not custom live integrations.

Each Gmail rule belongs to a user-configured Money Source and defines how that source searches for evidence. CanCan may provide a useful default rule for a supported provider, and the user may edit or override its query.

A Gmail rule may have a provider hint, but downloaded documents still go through classifier/parser verification. Do not trust the rule or source assignment alone.

## Data model

```text
gmail_search_rules
- id
- money_source_id
- name
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

## Storage policy

Use minimum storage.

Default:

```text
store message id
store thread id if useful
store sender/from
store subject
store date/internalDate
store attachment metadata
store downloaded attachment bytes in file vault
store file hash and source document metadata
```

Do not store full email body by default unless needed for evidence/parser behavior or explicitly enabled.

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
```

## Acceptance criteria

- Gmail connect uses Desktop OAuth + PKCE + loopback redirect.
- No Gmail data touches a CanCan server.
- Refresh token is stored in local secret storage.
- Only read-only Gmail scope is requested.
- User can create guided rules and edit expert query.
- Every rule belongs to a configured Money Source; supported providers may supply editable default rules.
- User can test a rule before enabling it.
- App supports startup/wake/manual/configurable polling sync.
- Re-running sync does not duplicate already imported attachments.
- Password-protected PDFs can be detected, unlocked locally, and optionally tied to a saved secret reference.
- Errors are visible and actionable.
- Development/test credentials and test users are isolated from the production OAuth project.
- Public release does not advertise Gmail connection until the production consent screen, website disclosures, restricted-scope justification, and required Google verification are complete.
