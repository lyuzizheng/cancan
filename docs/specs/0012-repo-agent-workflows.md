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
- Docs and harness changes require semantic review by an agent that did not author the patch; without one, the gate cannot pass.
- A semantic reviewer may return `needs_design`, but must not decide unresolved product, financial, security, privacy, or irreversible data questions.
- App commands must not be invented before real package scripts and paths exist.
- App implementation is routed through a machine-checked vertical-slice manifest.
- Implementers, testers, and reviewers use the same generated slice context and implementation review packet.

## Current folder shape

```text
AGENTS.md
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

Roles, repeated product/security/UI/testing rules, generic report templates, placeholder plugin guidance, keyword-routing scripts, and copied priority lists are intentionally excluded.

## Deterministic gate

`.agents/scripts/agent-preflight.sh` must verify:

```text
required entry files exist
every shell script parses with bash -n
the docs-harness workflow parses as YAML and retains required triggers, paths, permissions, and commands
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
skill frontmatter is valid and skill names match directories
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

For changes under `docs/` or `.agents/`, root `AGENTS.md`, or the docs-harness workflow:

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

The implementation review packet combines that exact context with tracked and untracked changes. Reviewers must also receive the user's exact task, author assumptions, success criteria, and verification evidence; the packet cannot infer those.

## Future app-code gates

Once application code exists, add command-backed checks only when the referenced scripts are real:

```text
typecheck
unit tests
safe test DB reset
migration check
fixture/parser tests
integration tests
build/package
UI visual inspection
```

The harness should call existing package scripts rather than wrap them in redundant orchestration.

## Acceptance criteria

- A generic agent can enter through root `AGENTS.md` and find the canonical read order.
- Product truth and current priorities are not duplicated in `.agents/`.
- Every canonical spec is uniquely numbered and indexed.
- Broken links, stale harness references, invalid skill metadata, and shell syntax errors fail preflight.
- Broken slice dependencies, missing spec/ADR/blocker references, missing test gates, and context-parity drift fail preflight.
- Harness self-tests prove that representative faults are detected.
- GitHub pull requests and pushes to `main` that change docs or harness files run the deterministic gate.
- Docs and harness changes require an independent semantic verdict.
- Semantic review distinguishes mechanical fixes from `needs_design` questions.
- No workflow claims app commands that do not exist.
- The real developer-setup test runs through preflight/CI and the harness self-test proves that version-pin drift is rejected.
