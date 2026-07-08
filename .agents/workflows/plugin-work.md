# Plugin And Connector Workflow

Use this for Gmail, manual import, watched folder, API/source connector, or future plugin work.

## Steps

1. Read `docs/specs/0003-gmail-collector.md` for Gmail work.
2. Read `docs/specs/0004-parser-contract.md` for parser handoff.
3. Read `docs/specs/0014-money-overview-source-taxonomy.md` for source/account terminology.
4. Preserve read-only connector behavior.
5. Store raw evidence and metadata before deriving records.
6. Route unsupported mappings to review instead of silently committing.
7. Add mocked fixtures before claiming provider support.
8. Verify privacy and secret handling.

## Connector Safety

Connectors may read evidence. They must not send money, pay bills, place trades, withdraw assets, mutate Gmail, bypass MFA, or read secrets outside scoped APIs.
