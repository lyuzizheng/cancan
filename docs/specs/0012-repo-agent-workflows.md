# 0012. Repo Agent Harness Spec

## Goal

Define a small repo-local harness that helps coding agents read the right sources, execute repeatable workflows, detect structural documentation drift, and request independent semantic review without duplicating product truth.

## Stable decisions

- Root `AGENTS.md` is the tool-neutral entry point.
- `docs/STRUCTURE.md` is the only source contract.
- `docs/specs/README.md` is the only maintained spec index.
- `.agents/` owns procedure, not product decisions or current priorities.
- Skills are trigger-oriented entry points; workflows own detailed task loops.
- Deterministic checks are blocking and must be dependency-light.
- The real deterministic docs harness runs locally and in GitHub Actions.
- Changes in the scope defined by `.agents/docs-semantic-review.md` require review by an agent that did not author the patch; without one, the gate cannot pass.
- A semantic reviewer may return `needs_design`, but must not decide unresolved product, financial, security, privacy, or irreversible data questions.
- App commands must not be invented before real package scripts and paths exist.
- App implementation is routed through a machine-checked vertical-slice manifest.
- Implementers, testers, and reviewers use the same generated slice readiness and canonical source index. Compact transport never reduces evidence: each role opens every indexed source in full from the exact working tree and head and records the inspected head and paths. The compact implementation review packet inventories the stable change and required handoff while each role reads source content and the cumulative diff directly from the shared repository.
- Project-scoped custom agents pin executable role, model, reasoning, and intentional subagent permission defaults without duplicating workflow or product truth. Main-agent permissions remain user/session-owned, and the implementer imposes no repo-level sandbox default. The parent turn's live permission selection is reapplied to every child and may supersede any subagent default.
- Execution is sized by consequence. Fast PR-comment maintenance stays in the root thread with focused checks and PR CI. Standard work adds at most one independent tester or reviewer when it materially improves evidence. High-risk financial/data, migration, security/privacy/secret, auto-commit, release/update, or agent-harness work keeps one production-code writer, its applicable independent review, and one final full relevant local gate. A separate tester is required only when the user requests it or execution independence changes the evidence. Complex structural analysis may use the optional read-only explorer.
- Independent review has separate correctness/safety and critical-cleanup gates. Cleanup rejects unjustified complexity, incomplete replacements, diff-created orphans, dirty package/API boundaries, unexplained magic logic, and tests that miss the active path.
- Required independent code review begins after the cumulative diff passes focused pre-review checks. Findings return to the production-code writer, affected focused checks rerun, and the reviewer re-reviews the entire cumulative diff. The expensive final app gate runs once after code review passes; unrelated full builds are not repeated after each edit. Docs/harness changes instead run deterministic preflight and harness self-test before their required semantic review.

## Current folder shape

```text
AGENTS.md
.codex/
  config.toml
  agents/
    explorer.toml
    implementer.toml
    tester.toml
    reviewer.toml
.github/workflows/
  application.yml
  application-native.yml
  docs-harness.yml
.agents/
  README.md
  ROUTER.md
  docs-semantic-review.md
  workflows/
    development-cycle.md
    design-grill.md
    implement-feature.md
    issue-delivery.md
    file-issues.md
    fix-issue.md
    plugin-work.md
    refine-architecture.md
    refine-ui.md
    review-code.md
    simulated-testing.md
  skills/
    cancan-architecture-refinement/
    cancan-code-review/
    cancan-design-grill/
    cancan-docs-orientation/
    cancan-implementation-cycle/
    cancan-issue-triage/
    cancan-issue-filing/
    cancan-issue-fixing/
    cancan-testing-simulation/
    cancan-ui-quality/
  scripts/
    agent-preflight.sh
    check-agent-skills.sh
    check-codex-agents.sh
    check-ci-workflow.sh
    check-docs-consistency.sh
    check-implementation-slices.sh
    check-links.sh
    check-spec-index.sh
    context-for-slice.sh
    docs-review-packet.sh
    harness-self-test.sh
    implementation-review-packet.sh
    implementation-slices.rb
    new-spec.sh
```

Narrative role layers, repeated product/security/UI/testing rules, generic report templates, placeholder plugin guidance, keyword-routing scripts, and copied priority lists are intentionally excluded. `.codex/agents/` contains only executable bindings; detailed loops stay in `.agents/workflows/`. Exception: the `issue-delivery` workflow carries its own category/priority/scope rubric as in-workflow policy (declared canonical in that file); it duplicates no spec truth.

## Deterministic gate

`.agents/scripts/agent-preflight.sh` must verify:

```text
required entry files exist
every shell script parses with bash -n
the docs-harness workflow parses as YAML and retains required triggers, paths, permissions, and commands
the Linux fast-application workflow parses as YAML and retains broad application source paths, read-only permissions, pinned Node/package-manager inputs, preflight, frozen pnpm resolution, superseded-run cancellation, and the real fast root verification command
the macOS native-application workflow parses as YAML and retains only native/sidecar/parser/migration/toolchain source paths, read-only permissions, draft-PR suppression, manual execution, superseded-run cancellation, preflight, frozen pnpm/Cargo resolution, bounded Cargo caching, and the real single-sidecar native root verification command
spec numbers are unique
every spec has required headings
every spec appears exactly once in docs/specs/README.md
the spec index contains no missing files
local Markdown links resolve
implementation slice IDs and dependencies are valid and acyclic
slice readiness/status agrees with active blockers and completed dependencies
ready, in-progress, and completed slices use only Accepted required ADRs
slice spec/ADR paths and active blocker names resolve
every active P0/P1 alignment area is referenced by at least one non-completed slice
every slice declares packages/surfaces, test gates, and an outcome
implementation review executes the same generated readiness validation as implementation/testing, rejects unknown slices, and points to the canonical source index without embedding that index, full source files, or the cumulative diff in the review packet
the development/review loop retains preflight, root verification, a unique critical-cleanup gate, and cumulative-diff re-review after fixes
skill frontmatter is valid and skill names match directories
project agent files retain their required model, reasoning, explicit subagent permission defaults, and implementer omission of a repo-local sandbox default
the reviewer binding delegates detailed judgment to the canonical review workflow
private fixtures are not tracked
removed harness layers are not referenced
current priorities are not copied into .agents
ADR statuses use the allowed vocabulary
the current tree and pending diff have no whitespace errors
developer-setup scripts parse, pinned toolchain versions agree, checksums reject tampering, platform routing is explicit, foreign tool paths are preserved, and shell-profile updates are idempotent
```

`.agents/scripts/harness-self-test.sh` copies the docs/harness to a temporary repository and injects known faults. The self-test passes only when the relevant gate rejects each fault.

## Semantic gate

Deterministic scripts cannot reliably detect contradictions such as two valid specs assigning different navigation or lifecycle behavior.

For changes under `docs/`, `.agents/`, or `.codex/`, root `AGENTS.md`, or the docs-harness workflow:

```text
run deterministic preflight
generate a packet with .agents/scripts/docs-review-packet.sh <base>
give the packet to a reviewer that did not author the patch
apply mechanical findings
leave judgment-dependent findings as explicit needs_design blockers
rerun both gates
```

The judge contract and required output shape live in `.agents/docs-semantic-review.md`.

## Implementation context gate

`docs/agent/implementation-slices.md` is the machine-checked implementation plan. It maps each vertical slice to the minimum required specs/ADRs, exact active blockers, dependencies, packages/surfaces, test evidence, and outcome.

```text
.agents/scripts/context-for-slice.sh <slice-id>
.agents/scripts/implementation-review-packet.sh <slice-id> [base]
```

The context generator always identifies root instructions, the source contract, current state, active alignment register, and only the selected slice's canonical specs/ADRs. It emits exact paths and heading line numbers rather than copying or summarizing canonical content; before acting, every role opens every indexed source in full from the exact working tree and head and records that evidence. It must label each slice `STOP`, `EVIDENCE ONLY`, `READY`, or `COMPLETE`. `EVIDENCE ONLY` permits the named disposable spike/test work needed to resolve blockers, never production implementation or downstream work.

The implementation review packet validates the selected slice, then identifies it plus exact base/head commits, a content-sensitive working-tree fingerprint, tracked/untracked inventory, compact diff stat, rename/deletion evidence, and commands for inspecting the complete cumulative diff directly from the shared repository. Before review, regenerate the packet and require its head and fingerprint to match; then inspect the exact base-to-head diff, post-head working-tree diff, and every indexed untracked file. The packet points to the separately generated source index instead of embedding that index, file content, or the full diff. It also names the required external handoff: the user's exact task, author assumptions and scope, success criteria, selected execution tier and justification, every canonical source inspected at the exact head commit, exact verification commands/results, UI evidence when relevant, and previous findings/resolutions for re-review. The packet cannot infer those inputs.

## Subagent execution boundary

The root agent owns orchestration and final reporting and normally owns production-code writes. It selects the smallest justified path:

```text
Fast     root writer -> focused checks -> push -> PR CI
Standard root writer -> focused checks -> optional one independent role -> final applicable check
High     one writer -> focused checks -> applicable independent review -> one final full relevant gate
```

Do not run multiple source-writing agents concurrently. A separate implementer is optional, not mandatory. A tester is used only when requested or when an independent environment, UI execution, database state, or other execution boundary materially changes the evidence; otherwise the writer runs deterministic commands and the independent reviewer judges the cumulative diff. Docs/harness changes run preflight and harness self-test before the semantic reviewer required by `.agents/docs-semantic-review.md`, but do not run unrelated application builds. UI inspection is required for user-visible behavior only.

An independent role receives a compact handoff containing the exact task, slice ID, review base, selected execution tier and justification, assumptions, success criteria, every canonical source inspected at the exact head commit, verification evidence, and prior findings. It must not require a copy of the root transcript or tool history because the repository, full contents of every indexed canonical source, and cumulative diff are the shared evidence. For pull-request work, collect available review findings while the PR is draft, route them to the sole writer as one batch, and mark the PR ready only for final CI evidence.

Review findings return to the sole writer. The writer reruns affected focused checks, and the reviewer must re-review the entire cumulative diff. The full selected-slice or root application gate runs after review passes rather than before every review round. If that final gate requires a code fix, the changed cumulative diff returns to review and the failed/final gate reruns.

The executable bindings live in `.codex/agents/`. `.codex/config.toml` caps agent nesting at direct children so workers cannot create an uncontrolled hierarchy; it intentionally does not copy personal `approval_policy` or `sandbox_mode` values into the repo. Explorer/reviewer declare read-only defaults, tester declares workspace-write, and implementer omits `sandbox_mode`. The parent turn's live permission selection is reapplied to every child regardless of these defaults, so explorer/reviewer no-edit behavior is enforced by their role/workflow instructions and independent-authorship rule rather than claimed as hard sandbox isolation. `.agents/scripts/check-codex-agents.sh` rejects role, model, reasoning, permission-default ownership, or concurrency drift; it does not claim to validate a future session's effective runtime sandbox.

## Application-code gates

The app foundation introduced these real root commands:

```text
pnpm typecheck
pnpm test:unit
pnpm check:rust
pnpm test:rust
pnpm build:web
pnpm build:desktop
pnpm verify:fast
pnpm verify:native
pnpm verify
```

`.github/workflows/application.yml` runs preflight and `pnpm verify:fast` on Linux for every application pull-request revision and push to `main`. `.github/workflows/application-native.yml` runs `pnpm verify:native` on macOS only when native desktop, sidecar/parser, migration, dependency, or pinned-toolchain inputs change; draft pull requests skip the job, ready pull requests run it, and maintainers may invoke it manually. Both workflows cancel superseded runs. The native gate restores Cargo registry/git/desktop-target cache data keyed by OS, the pinned toolchain, and the production Cargo lockfile. Its package-level orchestrator builds the sidecar once before Rust tests, clippy, and the Tauri build; standalone commands still prepare their own sidecar. The Rust suite remains authoritative native CI work rather than part of the default local `pnpm verify` gate. Later slices add safe test-DB reset, migration, fixture/parser, integration, and richer UI gates only when their implementations exist.

## Acceptance criteria

- A generic agent can enter through root `AGENTS.md` and find the canonical read order.
- Product truth and current priorities are not duplicated in `.agents/`.
- Every canonical spec is uniquely numbered and indexed.
- Broken links, stale harness references, invalid skill metadata, and shell syntax errors fail preflight.
- Custom agent model, reasoning, permission-default ownership, and nesting drift fails preflight.
- Broken slice dependencies, missing spec/ADR/blocker references, missing test gates, and context-parity drift fail preflight.
- Slice/review handoffs validate the selected slice and index the exact canonical sources and content-fingerprinted stable change without copying full source files, untracked content, or the cumulative diff; every role verifies the packet head/fingerprint, opens every indexed source in full, and records the inspected head and paths.
- Harness self-tests prove that representative faults are detected.
- GitHub pull requests and pushes to `main` that change docs, harness, or project agent configuration files run the deterministic gate.
- Docs, harness, and project agent configuration changes require an independent semantic verdict.
- Semantic review distinguishes mechanical fixes from `needs_design` questions.
- Work uses the smallest consequence-based execution tier, never multiple source-writing agents, and only the independent roles justified by material evidence. High-risk work retains complete final applicable evidence and independent review of the cumulative diff.
- No workflow claims app commands that do not exist.
- The real developer-setup test runs through preflight/CI and the harness self-test proves that version-pin drift is rejected.
- The fast/native application workflows, trigger boundaries, root verification composition, bounded Cargo cache, single-sidecar native orchestration, standalone command safety, and setup's production-plus-spike gate sequence are machine-checked; fault injection proves that removing safety coverage, widening native triggers to renderer-only changes, running native CI for drafts, rebuilding the sidecar inside the native gate, caching sidecar artifacts, or dropping superseded-run cancellation is rejected.
