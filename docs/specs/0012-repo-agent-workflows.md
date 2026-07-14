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
- Implementers, testers, and reviewers use the same generated slice context and implementation review packet.
- Project-scoped custom agents pin executable role, model, reasoning, and intentional subagent permission defaults without duplicating workflow or product truth. Main-agent permissions remain user/session-owned, and the implementer imposes no repo-level sandbox default. The parent turn's live permission selection is reapplied to every child and may supersede any subagent default.
- Non-trivial app work uses one production-code writer, an independent tester, and an independent read-only reviewer. Complex planning, exploration, document-conflict analysis, redesign, refactoring, performance analysis, and architecture optimization use the optional read-only explorer.
- Independent review has separate correctness/safety and critical-cleanup gates. Cleanup rejects unjustified complexity, incomplete replacements, diff-created orphans, dirty package/API boundaries, unexplained magic logic, and tests that miss the active path.
- Review begins only after the frozen diff passes every selected-slice test gate, repository preflight, root application verification, and any triggered UI or harness evidence. A later file change invalidates that evidence and requires full applicable testing plus review of the entire cumulative diff.

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
.github/workflows/docs-harness.yml
.agents/
  README.md
  ROUTER.md
  docs-semantic-review.md
  workflows/
    development-cycle.md
    design-grill.md
    implement-feature.md
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

Narrative role layers, repeated product/security/UI/testing rules, generic report templates, placeholder plugin guidance, keyword-routing scripts, and copied priority lists are intentionally excluded. `.codex/agents/` contains only executable bindings; detailed loops stay in `.agents/workflows/`.

## Deterministic gate

`.agents/scripts/agent-preflight.sh` must verify:

```text
required entry files exist
every shell script parses with bash -n
the docs-harness workflow parses as YAML and retains required triggers, paths, permissions, and commands
the macOS application workflow parses as YAML and retains required source paths, read-only permissions, pinned-toolchain inputs, preflight, frozen pnpm/Cargo resolution, and the real root verification command
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
implementation review uses the same generated context as implementation/testing
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

The context generator always includes root instructions, the source contract, current state, active alignment register, and only the selected slice's canonical specs/ADRs. It must label each slice `STOP`, `EVIDENCE ONLY`, `READY`, or `COMPLETE`. `EVIDENCE ONLY` permits the named disposable spike/test work needed to resolve blockers, never production implementation or downstream work.

The implementation review packet combines that exact context with tracked and untracked changes, a compact diff stat, and rename/deletion evidence. It also names the required external handoff: the user's exact task, author assumptions and scope, success criteria, exact verification commands/results, and UI evidence when relevant. The packet cannot infer those inputs.

## Subagent execution boundary

The root agent owns orchestration and final reporting. For non-trivial app changes:

```text
optional read-only explorer for complex planning and structural analysis
one implementer as the only production-code writer
stable diff
independent tester running every selected-slice gate plus preflight and root verification, with triggered UI/harness evidence
independent read-only reviewer applying correctness/safety and critical-cleanup gates to the full diff plus evidence
findings return to the implementer; any file change restarts full applicable testing and entire cumulative-diff review
```

Do not run multiple source-writing agents concurrently. A tester may write tests only when the root task explicitly delegates test authoring; otherwise it reports reproducible failures. UI inspection is required for user-visible behavior, not for unrelated backend-only changes.

The executable bindings live in `.codex/agents/`. `.codex/config.toml` caps agent nesting at direct children so workers cannot create an uncontrolled hierarchy; it intentionally does not copy personal `approval_policy` or `sandbox_mode` values into the repo. Explorer/reviewer declare read-only defaults, tester declares workspace-write, and implementer omits `sandbox_mode`. The parent turn's live permission selection is reapplied to every child regardless of these defaults, so explorer/reviewer no-edit behavior is enforced by their role/workflow instructions and independent-authorship rule rather than claimed as hard sandbox isolation. `.agents/scripts/check-codex-agents.sh` rejects role, model, reasoning, permission-default ownership, or concurrency drift; it does not claim to validate a future session's effective runtime sandbox.

## Application-code gates

The app foundation introduced these real root commands:

```text
pnpm typecheck
pnpm test:unit
pnpm check:rust
pnpm build:web
pnpm build:desktop
pnpm verify
```

`.github/workflows/application.yml` runs preflight plus `pnpm verify` on macOS. Later slices add safe test-DB reset, migration, fixture/parser, integration, and richer UI gates only when their implementations exist. The harness calls existing package scripts rather than wrapping them in redundant orchestration.

## Acceptance criteria

- A generic agent can enter through root `AGENTS.md` and find the canonical read order.
- Product truth and current priorities are not duplicated in `.agents/`.
- Every canonical spec is uniquely numbered and indexed.
- Broken links, stale harness references, invalid skill metadata, and shell syntax errors fail preflight.
- Custom agent model, reasoning, permission-default ownership, and nesting drift fails preflight.
- Broken slice dependencies, missing spec/ADR/blocker references, missing test gates, and context-parity drift fail preflight.
- Harness self-tests prove that representative faults are detected.
- GitHub pull requests and pushes to `main` that change docs, harness, or project agent configuration files run the deterministic gate.
- Docs, harness, and project agent configuration changes require an independent semantic verdict.
- Semantic review distinguishes mechanical fixes from `needs_design` questions.
- Non-trivial app work has one production-code writer, complete applicable testing evidence, and independent correctness/cleanup review of the entire cumulative diff.
- No workflow claims app commands that do not exist.
- The real developer-setup test runs through preflight/CI and the harness self-test proves that version-pin drift is rejected.
- The application workflow, root verification composition, and setup's production-plus-spike gate sequence are machine-checked; fault injection proves that removing any of those gates is rejected.
