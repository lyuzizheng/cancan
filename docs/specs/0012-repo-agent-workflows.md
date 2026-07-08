# 0012. Repo Agent Workflows Spec

## Goal

Plan repo-local agent workflows, skills, rules, and automation helpers that future AI coding agents can use to build CanCan reliably.

The user requested a repo-level agent folder later. Use `.agents/` as the preferred convention unless the chosen tool requires a different exact folder name.

## Current folder shape

```text
.agents/
  README.md
  ROUTER.md
  rules/
    product.md
    docs-consistency.md
    ui-quality.md
    testing.md
    security-data.md
  workflows/
    development-cycle.md
    design-grill.md
    implement-feature.md
    plugin-work.md
    review-code.md
    simulated-testing.md
    refine-architecture.md
    refine-ui.md
  roles/
    architect.md
    code-reviewer.md
    design-griller.md
    implementer.md
    qa-simulator.md
    ui-polisher.md
  skills/
    cancan-architecture-refinement/
    cancan-code-review/
    cancan-design-grill/
    cancan-docs-orientation/
    cancan-implementation-cycle/
    cancan-testing-simulation/
    cancan-ui-quality/
  plugins/
    README.md
    capability-contract.md
  scripts/
    agent-preflight.sh
    check-agent-skills.sh
    check-docs-consistency.sh
    new-spec.sh
    role-for-prompt.sh
  templates/
```

The current `.agents/` folder is text-first because application code has not started. It may define required future loops, but it must not pretend app build/test/DB/UI commands exist before package scripts and app paths are real.

## Required workflow concepts

### implement-feature

Reads docs, creates plan, implements one slice, runs tests, updates docs.

### reset-db

Drops/recreates test DB, runs migrations, seeds fixtures.

Status: future app-code workflow. Do not wire until database commands exist.

### inspect-ui

Starts app/dev server, opens UI with best available browser/computer-use/Chrome MCP/Playwright workflow, captures screenshots, checks visual issues, iterates.

Status: future app-code workflow. Current UI workflow documents the required inspection standard.

### package-app

Builds/packages app for target OS and verifies artifact exists.

Status: future app-code workflow. Do not wire until package commands exist.

### docs-consistency

Checks whether changed behavior requires docs/spec/progress updates.

## Tooling flexibility

The exact browser/computer-use tool may change over time.

Acceptable future tools include:

```text
computer-use
browser-use
Chrome MCP
Playwright screenshots
repo-installed skill scripts
```

The workflow should say "use the best available current tool" rather than hard-coding one fragile dependency.

## Acceptance criteria

- `.agents/` workflows are created once real commands exist.
- Current text workflows do not claim fake package scripts or app paths.
- Future command-backed workflows reference actual package scripts and app paths.
- UI workflows require screenshot/visual inspection once UI exists.
- DB workflows include reset + migration + seed once DB exists.
- Docs workflows enforce updates to `docs/agent/` and `docs/specs/`.
