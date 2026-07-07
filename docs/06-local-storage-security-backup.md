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
  OCR/text/layout outputs
backups/
```

## SQLite strategy

Use SQLite as the local source of truth. Prefer SQLCipher or another full-database encryption strategy.

SQLite stores:

```text
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

Use content-addressed storage.

```text
files/
  aa/bb/<sha256>.pdf
  cc/dd/<sha256>.png
  ee/ff/<sha256>.json
```

SQLite stores:

```text
file_hash
file_path
mime_type
size
created_at
source metadata
```

Advantages:

```text
document dedupe
reparse by hash
backup manifest/checksum
separate file encryption possible
less database bloat
```

## Secret storage

Do not store API keys or OAuth refresh tokens in SQLite.

Secrets should be stored in OS secret storage / Tauri Stronghold / platform keychain.

Examples:

```text
OpenAI API key
Anthropic API key
GLM/Kimi/DeepSeek provider keys
Gmail OAuth refresh token
Wise API token
Bitget read-only API token
Moomoo credentials/token metadata
vault encryption key
```

React UI should only know whether a secret is configured, not the actual secret value.

## AI provider keys

The app may support:

```text
OpenAI-compatible API
OpenAI
Anthropic
Gemini
DeepSeek
GLM
Kimi
local model endpoint
Ollama/OpenAI-compatible local endpoint
```

Config table can store non-secret metadata:

```text
provider_name
base_url
model_name
enabled
rate_limit
cost_tracking_enabled
```

Secret store keeps keys.

## Backup strategy

Do not put the live SQLite database directly in iCloud Drive and write to it there.

Recommended flow:

```text
local live vault
-> checkpoint/consistent SQLite backup snapshot
-> manifest + checksums
-> encrypted .financevault bundle
-> copy to iCloud backup folder
```

## Backup bundle shape

```text
2026-07-07_231500.financevault.zip
  manifest.json
  finance.sqlite
  files/
    aa/bb/<sha256>.pdf
    cc/dd/<sha256>.png
    ee/ff/<sha256>.json
  audit.log
```

Example manifest:

```json
{
  "vault_version": 1,
  "created_at": "2026-07-07T23:15:00+08:00",
  "app_version": "0.1.0",
  "schema_version": 12,
  "device_id": "macbook-pro",
  "sqlite_sha256": "...",
  "file_count": 328,
  "encrypted": true
}
```

## Backup modes

### MVP: backup/restore

```text
Mac app is the primary writer.
iCloud stores encrypted snapshots.
Mobile app can later read latest snapshot.
No multi-writer conflict resolution yet.
```

### Future: command sync

Mobile can write small review commands:

```json
{
  "device": "iphone",
  "commands": [
    {
      "type": "confirm_match",
      "match_edge_id": "abc",
      "confirmed_at": "2026-07-07T12:00:00+08:00"
    }
  ]
}
```

Desktop app merges commands later.

### Future: true sync

Requires:

```text
event log
record versions
conflict resolution
soft delete
merge rules
CloudKit or custom sync
```

This is not MVP.

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
