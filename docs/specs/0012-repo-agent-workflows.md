# 0012. Repo Agent Workflows Spec

## Goal

Plan repo-local agent workflows, skills, rules, and automation helpers that future AI coding agents can use to build CanCan reliably.

The user requested a repo-level agent folder later. Use `.agents/` as the preferred convention unless the chosen tool requires a different exact folder name.

## Planned folder shape

```text
.agents/
  README.md
  rules/
    product.md
    docs-consistency.md
    ui-quality.md
    testing.md
  workflows/
    implement-feature.md
    run-tests.md
    reset-db.md
    inspect-ui.md
    package-app.md
  skills/
    cancan-docs-orientation/
    cancan-ui-inspection/
    cancan-db-reset/
    cancan-release-check/
```

Do not create these workflow files blindly before app code exists. Create them when they can reference real commands.

## Required workflow concepts

### implement-feature

Reads docs, creates plan, implements one slice, runs tests, updates docs.

### reset-db

Drops/recreates test DB, runs migrations, seeds fixtures.

### inspect-ui

Starts app/dev server, opens UI with best available browser/computer-use/Chrome MCP/Playwright workflow, captures screenshots, checks visual issues, iterates.

### package-app

Builds/packages app for target OS and verifies artifact exists.

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
- Workflows reference actual package scripts and app paths.
- UI workflows include screenshot/visual inspection.
- DB workflows include reset + migration + seed.
- Docs workflows enforce updates to `docs/agent/` and `docs/specs/`.
