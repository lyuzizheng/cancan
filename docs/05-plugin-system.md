# 05. Plugin System

## Goal

Plugins allow CanCan to collect data from different financial sources without hardcoding every source into the core app.

A plugin can discover documents, fetch API records, parse files, create staged records, and report sync status.

## Plugin categories

### API connector plugins

Examples:

```text
Wise
Moomoo
Bitget
future Revolut/Bank API
```

Responsibilities:

```text
read credentials through secret manager
fetch incremental data with cursor/overlap window
save raw API JSON snapshots
normalize API records into external_records
never perform write/payment/trade operations
```

### Gmail connector plugin

Responsibilities:

```text
search finance emails
filter by sender/subject/date/attachment
save email metadata
save attachments into file vault
route attachments to provider parsers
resume from Gmail history/cursor when possible
```

The Gmail plugin is a collector, not a bank parser. Provider-specific parsing happens later.

### Manual upload plugin

Responsibilities:

```text
allow drag/drop files
compute file hash
dedupe already-imported files
classify document
start parse job
```

Supported initial file types:

```text
PDF
CSV
XLSX
PNG/JPEG screenshots
JSON exports
```

### Watched folder plugin

Responsibilities:

```text
scan a user-selected folder
import new files by hash
ignore unchanged files
support one-click rescan
```

### Screenshot/image plugin

Responsibilities:

```text
OCR image
classify screenshot
extract visible rows/amounts
stage records with low-confidence default
```

## Plugin interface

```ts
export interface SourcePlugin {
  id: string;
  name: string;
  version: string;
  sourceType: SourceType;

  setup?(ctx: PluginContext): Promise<PluginSetupResult>;
  healthCheck?(ctx: PluginContext): Promise<PluginHealth>;
  discover(ctx: PluginContext, cursor?: SyncCursor): Promise<DiscoveryResult>;
  ingest(ctx: PluginContext, item: DiscoveredItem): Promise<IngestResult>;
  parse?(ctx: PluginContext, sourceDocumentId: string): Promise<ParseResult>;
  normalize?(ctx: PluginContext, parseRunId: string): Promise<NormalizeResult>;
}
```

## Plugin context

Plugins get limited tools, not full app access.

```ts
export interface PluginContext {
  vault: VaultDocumentWriter;
  db: PluginScopedDatabase;
  secrets: SecretReader;
  logger: PluginLogger;
  ai?: AIProviderRouter;
  clock: Clock;
}
```

A plugin should not get raw SQL access unless scoped and audited.

## Plugin lifecycle

```text
installed
-> configured
-> enabled
-> scheduled/manual run
-> discover
-> ingest
-> parse
-> normalize
-> stage
-> reconcile
-> review/commit
```

## Incremental sync

Each plugin stores sync state.

```text
plugin_sync_states
- plugin_id
- finance_source_id
- cursor_json
- last_success_at
- last_overlap_start
- last_error_json
```

Use overlap windows for financial data because statements and posted transactions can arrive late.

Suggested default:

```text
API sources: re-fetch last 30-60 days
Email sources: search since last successful scan minus 7 days
Manual/watched folders: hash-based dedupe
```

## Read-only rule

Plugins must be read-only for financial institutions.

Allowed:

```text
read transactions
read balances
read statements
read holdings
read API exports
read emails/attachments
```

Disallowed:

```text
send money
pay bills
place trades
withdraw crypto
delete emails
change bank settings
bypass MFA/CAPTCHA
```

## Initial plugin priority

MVP plugin order:

```text
1. Manual PDF/CSV upload
2. Generic PDF text/table parser
3. Gmail finance attachment collector
4. Wise export/API
5. Moomoo export/API
6. Bitget read-only API
7. One bank statement parser, then expand bank coverage
8. Manulife PDF/policy statement parser
```
