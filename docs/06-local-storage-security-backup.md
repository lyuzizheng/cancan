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
  raw email snapshots
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
sync_runs
settings metadata
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

Do not store API keys, OAuth refresh tokens, or vault keys in plain SQLite.

Secrets should be stored in OS secret storage / Tauri Stronghold / platform keychain.

Examples:

```text
OpenAI API key
Anthropic API key
Gemini/DeepSeek/GLM/Kimi provider keys
Gmail OAuth refresh token
Wise API token
Bitget read-only API token
Moomoo token metadata
vault encryption key
```

React UI should only know whether a secret is configured, not the actual secret value.

## AI provider consent

AI providers are disabled by default until explicit setup.

Raw document upload to cloud AI should require a clear consent model. Recommended v1 policy:

```text
user configures provider key
user enables AI parsing for source or document type
app shows that source evidence may be sent to that provider
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

Backups should not include OAuth/API secrets by default. Restore should force re-authentication unless a future explicit encrypted-secret backup design is approved.

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
