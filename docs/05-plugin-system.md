# 05. Plugin System

## Goal

Plugins allow CanCan to collect financial evidence from different sources without hardcoding every source into the core app.

A plugin may discover documents, fetch read-only API records, ingest files, parse source-specific formats, normalize records, and report sync status.

## MVP plugin priority

The MVP order is now automation-first:

```text
1. Manual PDF/CSV upload as test harness
2. Gmail read-only collector with user-configured search rules
3. DBS bank account statement parser
4. DBS credit card statement parser
5. UOB bank account statement parser
6. UOB credit card statement parser
7. Wise PDF/CSV/export parser
8. Watched folder import
9. Wise API, then Moomoo/Bitget read-only APIs later
```

Manual import can be built first for testing, but Gmail automation is part of MVP success.

## Gmail collector plugin

Responsibilities:

```text
OAuth read-only Gmail access
store refresh token in OS secret storage
let user configure sender/subject/keyword/search rules
support has:attachment and filename/provider filters
support scan start date
support incremental search since last successful scan with overlap window
save email metadata
save attachments into encrypted file vault
route attachments to classifier/parser jobs
never delete or modify emails
```

The Gmail plugin is a collector, not a bank parser. Provider-specific parsing happens later.

Recommended Gmail rule shape:

```text
gmail_search_rules
- id
- name
- query
- required_keywords_json
- excluded_keywords_json
- start_after
- overlap_days
- enabled
- last_success_at
- cursor_json
```

## Manual upload plugin

Responsibilities:

```text
allow drag/drop files
compute file hash
dedupe already-imported files
classify document
start extraction/parse job
```

Supported initial file types:

```text
PDF
CSV
XLSX
PNG/JPEG screenshots
JSON exports
```

## API connector plugins

Examples:

```text
Wise
Moomoo
Bitget
future bank APIs
```

Responsibilities:

```text
read credentials through secret manager
fetch incremental read-only data with cursor/overlap window
save raw API JSON snapshots
normalize API records into external_records
never perform write/payment/trade operations
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

## Incremental sync

Use overlap windows because financial data arrives late.

Suggested defaults:

```text
Gmail sources: search since last successful scan minus 7 days
API sources: re-fetch last 30-60 days
Manual/watched folders: hash-based dedupe
```

## Read-only rule

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
