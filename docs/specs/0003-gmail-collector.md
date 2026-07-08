# 0003. Gmail Collector Spec

## Goal

Automatically collect financial statement evidence from Gmail while preserving local-first behavior.

## MVP approach

Use official Gmail read-only OAuth/API. Do not use browser computer-use automation for MVP unless the official API path is blocked.

CanCan remains local-first because:

- OAuth token is stored locally in OS secret storage;
- downloaded emails/attachments are stored in the local encrypted vault;
- CanCan has no hosted backend or proxy;
- Gmail is an external source, not CanCan storage.

## UX model

Use guided rules plus expert query editing.

Top section: guided builder.

```text
Provider: DBS | UOB | Wise | Custom
Document type: bank statement | credit card statement | export | custom
Required keywords
Excluded keywords
Has attachment toggle
Filename hints
Start date
Overlap days
Enabled toggle
```

Bottom section: expert Gmail query preview/editor.

The user can manually edit the generated query. If edited, preserve both:

```text
generated_query
manual_query_override
active_query
```

## Data model

```text
gmail_search_rules
- id
- name
- provider_hint
- document_type_hint
- required_keywords_json
- excluded_keywords_json
- generated_query
- manual_query_override
- active_query
- start_after
- overlap_days
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
- cursor_json
- last_error_json
- created_at
- updated_at
```

## Collector behavior

```text
1. Load enabled rules.
2. Build search window from start_after or last_success_at minus overlap_days.
3. Query Gmail read-only API.
4. Store email metadata.
5. Download matching attachments.
6. Hash attachments.
7. Dedupe by file hash and Gmail message/attachment id.
8. Create source_documents.
9. Queue extraction/parser jobs.
10. Update sync state.
```

## Security

Allowed Gmail actions:

```text
read message metadata
read message body when needed for evidence
read/download attachments
```

Disallowed Gmail actions:

```text
send email
modify labels
delete email
mark read/unread
change mailbox settings
```

## Tests

Use mocked Gmail API fixtures for:

```text
first scan from start date
incremental scan with overlap
duplicate attachment hash
same attachment from same email
rule disabled
OAuth/token error
no matching emails
PDF attachment import
CSV attachment import
```

## Acceptance criteria

- User can create a guided rule and see the expert query.
- User can override the expert query.
- Scan imports matching attachments into Library.
- Re-running scan does not duplicate already imported files.
- Errors create visible job/review status instead of silent failure.
