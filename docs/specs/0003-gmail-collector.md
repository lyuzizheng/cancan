# 0003. Gmail Collector Spec

## Goal

Automatically collect financial statement evidence from Gmail while preserving local-first behavior.

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

For development/MVP, Google Cloud side needs:

```text
Gmail API enabled
OAuth consent screen configured
OAuth client application type: Desktop app
redirect URI using loopback pattern
```

Early development can require bring-your-own Google OAuth client configuration if needed. A later public release may use a CanCan-owned OAuth client.

## Scope policy

Use minimum Gmail scope.

Required scope:

```text
https://www.googleapis.com/auth/gmail.readonly
```

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

A Gmail rule may have a provider hint, but downloaded documents still go through classifier/parser verification. Do not trust the rule alone.

## Data model

```text
gmail_search_rules
- id
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
- provider_key
- money_source_id
- account_id nullable
- document_type_hint
- secret_storage_key
- hint_label
- created_at
- updated_at
```

`statement_secret_refs` stores only references and hints. Actual statement passwords must live in OS secret storage / Keychain / Stronghold.

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
-> user may save password for matching provider/account/document type
-> extraction resumes
```

Rules:

- password entry is optional;
- password unlock happens locally;
- passwords must not be sent to AI providers;
- passwords must not be written to logs, parse payloads, raw_json, normalized_json, or backups by default;
- saved passwords are referenced by `statement_secret_refs` and stored in OS secret storage;
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
- User can test a rule before enabling it.
- App supports startup/wake/manual/configurable polling sync.
- Re-running sync does not duplicate already imported attachments.
- Password-protected PDFs can be detected, unlocked locally, and optionally tied to a saved secret reference.
- Errors are visible and actionable.
