# 0019. AI Capability Platform, Agent Gateway, and CLI

## Goal

Define one Phase 2 host-owned backend interface that exposes bounded CanCan capabilities to:

```text
CanCan's own in-app AI features
the CanCan Desktop renderer
the first-party CLI
external AI agents through strict function tools
```

The platform lets the renderer and AI-backed features use the same user-authorized product capabilities without creating parallel business interfaces or granting direct database, Vault-file, filesystem, secret, or ledger authority.

## Phase and implementation blocker

This is a Phase 2 design. Do not implement it through the active `AI capability platform and assistant scope` blocker or pull its infrastructure into Phase 1 hardening, public preview, Gmail, or backup work.

In-app assistant history/settings, external approval-policy persistence, audit retention, and statement-coverage provider retention/cost, confidence, and explanation rules remain unresolved in the active alignment register. The full first-party CLI target, external transport, default-off posture, enablement, token scope/revocation, host-owned approval boundaries, and exact coverage snippet bounds below are already accepted.

## Stable decisions

- The unlocked CanCan desktop process remains the sole Vault owner and privileged host, whether or not a renderer window is visible.
- One host-owned typed capability/service registry is the canonical backend interface shared by the Desktop renderer, in-app AI, internal scheduled jobs, first-party CLI, and strict external-agent adapters.
- The Phase 2 target catalog covers every existing user-facing read outcome and every host-validated edit, link, commit, delete, and restore mutation. Delivery may migrate that catalog in verified increments, but it must not leave a permanent renderer-only business API or create a narrower second AI/CLI implementation.
- Tauri IPC, trusted in-process calls, the local socket, CLI commands, and strict function tools are transport/caller adapters over that interface. They do not implement a second renderer-only or agent-only business contract.
- Capabilities are purpose-specific product operations, not generic CRUD, SQL, filesystem, or `get all source data` access.
- App-internal AI and external agents use the same capability implementations and schemas; they may have different caller visibility and approval policy.
- AI results are advisory unless a registered mutation capability explicitly requires and receives host-validated user approval.
- External access is off by default.
- External adapters connect only to the running unlocked Desktop app through a user-only local Unix-domain socket. Phase 2 adds no TCP listener, hosted gateway, LaunchAgent, background daemon, or second Vault owner.
- When the CanCan process is not running or the Vault is manually locked, no AI capability or scheduled analysis runs. Closing the last renderer window alone may retain the minimal unlocked Rust runtime; startup/unlock may catch up overdue internal work idempotently.
- The CLI is a thin client of the capability registry. It does not open the Vault database or decrypt source files itself.
- Secrets remain behind the privileged host and are never returned through a capability result.
- Every capability invocation is bounded, schema-validated, caller-attributed, redacted, and auditable.

## Capability registry

Each registered capability declares enough policy for every adapter to enforce the same product boundary:

```text
stable versioned name
description
strict input and output schema
allowed caller classes
required unlocked/read scope
effect: read | propose | mutate
approval rule
idempotency rule for mutations
result bounds and redaction
audit event shape
```

Caller classes distinguish at least:

```text
desktop renderer
in-app AI
first-party CLI
external agent
internal scheduled job
```

A capability can be visible to in-app AI without being externally exposed. External visibility never follows automatically from renderer or internal-AI use.

## API design rule

Before adding a capability, search the registry and the existing host service contract. Reuse or version the existing capability when the user outcome, authority, and effect are the same. A new transport, caller class, UI surface, model, or scheduled trigger is not a reason to create a second capability.

Create a new capability only when a concrete outcome needs a materially different input/output schema, authority scope, or read/propose/mutate effect. Keep one implementation and add caller/transport adapters around it. Do not add per-feature registries, wrapper services, capability factories, generic repositories, or parallel `v2` APIs merely to prepare for future flexibility.

Prefer:

```text
get_asset_summary()
get_monthly_summary(month)
list_review_items(status)
explain_money_flow(chain_id)
search_transactions(query)
get_source_updated_at()
analyse_statement_coverage(scope)
edit_review_item(item_id, expected_version, patch)
link_records(record_ids, expected_versions, proposal)
commit_review_items(item_ids, expected_versions)
delete_source_file(document_id, expected_version)
restore_source_file(document_id, expected_version)
```

Reject interfaces such as:

```text
query_database(sql)
read_vault_file(path)
get_all_source_data()
invoke_tauri_command(name, payload)
run_shell(command)
```

The capability implementation may call internal repositories and services, but its public schema is a bounded presentation/domain contract. It returns opaque product identifiers and source references only where the caller needs them. Existing Phase 1 renderer commands may remain until their owning Phase 2 capability is introduced; each migrated feature then removes or reduces the old command to a thin adapter rather than maintaining two authorities.

## In-app AI

CanCan's own AI features call the registry through a trusted in-process adapter. Trusted means the app does not need an external socket token for itself; it does not mean the model receives privileged host access.

The in-app adapter:

- sends only the capability-specific input selected for that feature;
- exposes only capabilities allowed to the in-app AI caller class;
- validates model-proposed arguments before invocation;
- validates and bounds capability results before returning them to the model;
- keeps user approval outside the model for every mutation;
- records the model/provider/runtime and source references needed for an honest explanation.

Statement coverage is one future consumer. One invocation is scoped to exactly one Money Source, one child account, and one statement document type. The first input contains bounded structured period/processing metadata and record summaries. The model may make at most one follow-up request for extracted snippets from no more than three explicitly referenced in-scope source documents, two pages per document, and 8,000 extracted characters total. The host rejects arbitrary document access, full-source dumps, paths, cross-scope references, and every over-limit or second request. Snippets require the configured AI-data authorization and are not persisted in capability/job logs. Provider retention/cost and explanation/confidence rules remain blocked.

## CLI and external-agent gateway

The first-party CLI uses HTTP-style request semantics over a user-only Unix-domain socket owned by the running Desktop app. The socket is unavailable while the Vault is locked and removed or rejected on lock/exit.

The first-party AI CLI exposes the full caller-allowed user-facing capability catalog, including purpose-specific reads and host-validated edit/link/commit/delete/restore mutations. Registry caller visibility always excludes internal-only and scheduled-job-only capabilities from the CLI. “Full” does not mean generic CRUD, direct Tauri dispatch, repository access, SQL, filesystem access, or bypassed approvals. Each operation still uses its canonical schema, expected-version/idempotency rule, deterministic financial validation, caller scope, and host-owned approval policy.

The external gateway:

- is explicitly enabled in Settings;
- issues revocable local tokens with bounded capability scopes;
- shows the caller identity and requested effect;
- enforces `Ask`, `Allow`, or `Deny` policy per capability/scope;
- requires a fresh host-owned confirmation for high-consequence mutations;
- never lets an agent approve its own request;
- rejects stale, replayed, malformed, oversized, or unsupported requests;
- records a redacted audit entry without prompts, secrets, raw documents, or unrestricted financial payloads.

Read capabilities can support durable user policy after the policy model is accepted. Mutation capabilities default to `Ask`; permanent broad approval for financial mutations is not implied by this design.

## Scheduling

The durable local job system remains the only scheduler for product work. A capability-specific internal job can run while Desktop is available and can catch up overdue work at startup/unlock.

For statement coverage, accepted source/account/statement-period or processing-state changes mark the `(money_source_id, account_id, document_type)` scope due. At startup/unlock, CanCan evaluates a due scope only when it has no successful evaluation for the current local calendar day. Retries reuse the same logical run/idempotency scope. This is a trigger policy, not a provider cadence rule: the AI still analyses the bounded evidence to decide whether a period appears missing.

There is no separate AI cron daemon. Scheduling does not make routine internal runs visible as technical Jobs, but a useful result or actionable failure may project into Tasks and deep-link to the owning feature surface.

## Security and privacy boundaries

- No capability returns secrets, Keychain values, bookmarks, raw filesystem paths, SQL, unrestricted source bytes, or full-document plaintext by default.
- A feature that needs document content must define an explicit bounded content schema, disclosure, retention policy, and provider contract before implementation.
- The model never obtains a generic callback into Tauri commands, repositories, jobs, or the capability registry.
- External tokens are device-local, scoped, revocable, and excluded from backup.
- Lock, Vault switch, or permission revocation invalidates affected sessions and pending approvals.
- Logs and audit records use capability names, caller class, bounded identifiers, result status, timing, and redacted failure codes only.
- ADR 0002 remains authoritative: the agent proposes and explains; deterministic host rules and explicit user intent own financial mutation.

## Failure behavior

An unavailable AI provider, external gateway, or one capability must not clear or reject otherwise successful Overview, Review, Activity, or Sources data.

Return stable failures such as:

```text
vault_locked
capability_not_found
capability_not_allowed
approval_required
approval_denied
invalid_input
stale_request
provider_unavailable
result_too_large
```

Adapters may translate these codes into caller-appropriate copy, but they do not reinterpret a denial as success or retry a mutation with a new idempotency key.

## Acceptance criteria

- One registry definition generates or validates the renderer, internal-AI, scheduled-job, CLI, and strict-tool schemas without duplicate business implementations.
- The Phase 2 registry/CLI catalog covers all existing user-facing reads plus purpose-specific host-validated edit, link, commit, delete, and restore outcomes; migration removes or reduces each superseded renderer command to a thin adapter.
- Tauri, in-process, local-socket, CLI, and tool adapters call the same host-owned implementation; no adapter owns a parallel business rule.
- Registry review proves that a new entry is not a duplicate of an existing outcome/authority/effect contract; new callers and transports normally add adapters, not capabilities.
- Caller-visibility tests prove that an internal-only capability cannot be invoked by the CLI or an external agent.
- The in-app AI can use a mocked read capability but cannot access a repository, file, secret, or unregistered host command.
- External access is off by default, unavailable while locked, and uses only the user-only local socket.
- No daemon or TCP listener is installed; overdue internal work catches up idempotently after startup/unlock.
- Capability-specific source analysis does not expose a generic source-data dump.
- Statement coverage processes one Money Source/account/document-type scope per invocation; its single follow-up snippet request is capped at three documents, two pages per document, and 8,000 extracted characters total, cannot cross that scope, and cannot become stored job/log payloads.
- Read, proposal, and mutation policies are enforced consistently; an agent cannot approve its own mutation.
- Revocation, lock, Vault switch, stale request, replay, oversize input/result, and provider failure are deterministic and audited without sensitive payloads.
- CLI and strict function-tool adapters are thin translations over the same capability registry.
- Statement coverage uses the accepted scoped structured-plus-snippet payload and source/account-change due marker plus once-per-local-day startup/unlock catch-up policy; it remains blocked on cost/retention, confidence, and explanation rules.
