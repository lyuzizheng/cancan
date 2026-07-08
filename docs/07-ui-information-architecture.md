# 07. UI Information Architecture

## Product UI principle

CanCan is a polished asset-management and reconciliation app.

The main question is:

```text
What do I own, where is it, what changed, which source proves it, and what needs review?
```

## Visual direction

Use the `impeccable` design mindset for a production-grade desktop finance app: restrained, precise, trustworthy, visually polished, and operationally useful.

CanCan should have:

```text
left sidebar navigation
right main content area
compact but beautiful asset summaries
strong tables and review surfaces
source/account freshness indicators
clear action queues
future AI Assistant entry point
```

Avoid:

```text
giant fintech hero cards
decorative gradients
glassmorphism as default
busy card grids
cute illustrations
```

## Main navigation

```text
Command Center
Sources
Assets
Transactions
Reconciliation
Money Flow
Library
Jobs
AI Assistant
Settings
```

## Command Center

The MVP home screen should be an operational command center with asset summary embedded, not a pure dashboard.

Primary zones:

```text
left: persistent sidebar
main top: vault/source/sync status
main center: needs review, new evidence, failed parses, suggested links
main side/right rail: compact asset snapshot, freshness, backup status, assistant entry
```

Key metrics:

```text
Total net worth in base currency
Cash
Investments
Crypto
Insurance value
Liabilities
Unreconciled amount
Needs review count
New evidence count
Data freshness
Last Gmail scan
Last backup
```

## AI Assistant

Future assistant should use backend APIs/skills instead of raw database access.

Assistant can help with:

```text
monthly summary
asset analysis
source freshness questions
review item explanation
money-flow explanation
missing statement detection
spending/income summaries after ledger quality is high enough
```

Assistant cannot:

```text
read secrets
commit ledger directly
make payments
place trades
withdraw crypto
```

## Reconciliation UI

Review should not be overcomplicated. The default should be a scannable inbox/list with expandable detail.

Use side-by-side comparison when the item involves a candidate link:

```text
left record/event
right candidate record/event
proposed match type
confidence
rule evidence
AI explanation
actions: confirm, reject, edit, link manually
```

Keep AI explanation separate from deterministic evidence so the user can trust what is rule-based versus inferred.

## Money Flow

First UI version should be chain-first, not a complex graph canvas.

Example:

```text
UOB One Account -1000 SGD
-> DBS Visa payment
-> DBS Visa liability -1000 SGD
```

Backend should still model graph relationships so a node-edge visualization can be added later.

## MVP UI priority

Build in this order:

```text
1. Vault setup/unlock
2. Money source and sub-account setup
3. Manual import test harness
4. Gmail rule setup and scan status
5. Library document list/detail
6. Parser run detail
7. Staged records table
8. Review inbox
9. Command Center shell
10. Source detail and asset summary
11. Money Flow chain view
12. AI Assistant placeholder/tool surface
13. Backup settings
```

See `docs/specs/0006-command-center-ui.md` for implementation-grade UI requirements.
