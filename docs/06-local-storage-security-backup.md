# 06. Local Storage, Security, and Backup

## Storage goal

All sensitive finance data should live locally by default.

The vault contains:

```text
finance.sqlite
files/
  raw PDFs
  screenshots
  CSV/XLSX exports
  raw email snapshots when explicitly stored
  API JSON snapshots
  native text extraction output
  OCR/text/layout outputs
backups/
```

## SQLite strategy

Use SQLite as the local source of truth. SQLCipher or equivalent encryption should be mandatory from v1, even if the first implementation is simple.

SQLite stores:

```text
money_sources
accounts
instruments
source_documents metadata
parse_runs
external_records
ledger_events
ledger_legs
match_edges
review_items
jobs
settings metadata
secret reference metadata only
```

Avoid storing large PDFs/images as SQLite BLOBs unless there is a clear reason.

## File vault strategy

Use content-addressed storage:

```text
files/
  aa/bb/<sha256>.pdf
  cc/dd/<sha256>.png
  ee/ff/<sha256>.json
```

SQLite stores file metadata and hashes.

Advantages:

```text
document dedupe
reparse by hash
backup manifest/checksum
separate file encryption possible
less database bloat
```

## Secret storage

Do not store API keys, OAuth refresh tokens, PDF statement passwords, or vault keys in plain SQLite.

Secrets should be stored in OS secret storage / Tauri Stronghold / platform keychain.

Examples:

```text
AI provider API keys
Gmail OAuth refresh token
statement PDF password for DBS/UOB style documents
read-only source API token if a future supported provider needs one
vault encryption key material
```

React UI should only know whether a secret is configured, not the actual secret value.

SQLite may store secret references:

```text
secret_refs
- id
- secret_kind              -- ai_provider_key, gmail_refresh_token, statement_pdf_password, vault_key_ref
- scope_type               -- global, provider, money_source, account, document_type
- scope_id
- provider_key
- secret_storage_key
- hint_label
- created_at
- updated_at
```

Rules:

- no secret values in logs;
- no secret values in AI prompts;
- no secret values in parse payloads;
- no secret values in backups by default;
- user can remove saved secrets from Settings;
- restore should require re-authentication or re-entry unless a future encrypted-secret backup design is approved.

## AI provider consent

AI providers are disabled by default until explicit setup.

Raw document upload to cloud AI should require a clear consent model. Recommended v1 policy:

```text
user configures provider key or accepts default path
user enables AI parsing for source or document type
app shows that selected source evidence may be sent to that provider
parse outputs and prompt/version logs remain local
```

## Backup strategy

Do not put the live SQLite database directly in iCloud Drive and write to it there.

Recommended flow:

```text
local live vault
-> checkpoint/consistent SQLite backup snapshot
-> manifest + checksums
-> encrypted .financevault bundle
-> copy to iCloud backup folder or user-selected folder
```

## Backup bundle shape

```text
2026-07-07_231500.financevault.zip
  manifest.json
  finance.sqlite
  files/
    aa/bb/<sha256>.pdf
    cc/dd/<sha256>.json
  audit.log
```

Backups should not include OAuth/API secrets, AI keys, vault key material, or statement PDF passwords by default. Restore should force re-authentication or password re-entry unless a future explicit encrypted-secret backup design is approved.

## Version compatibility

Every vault and backup bundle should include version metadata.

```text
vault_schema_version
app_min_supported_schema_version
app_created_version
backup_format_version
created_at
```

Newer app reading older vault:

```text
run compatibility check
run allowed migrations
preserve source evidence and audit trail
show migration result before normal use if needed
```

Older app reading newer vault:

```text
block open or open read-only only if safe
show upgrade-required message
never silently downgrade schema
```

## Security posture

Default rules:

```text
read-only connectors only
no payment/trade/withdrawal capabilities
no LLM access to secrets
LLM can only propose parsed records and match candidates
all commits are deterministic and auditable
raw evidence is preserved
sensitive files encrypted at rest
backup encrypted before leaving local machine
```
