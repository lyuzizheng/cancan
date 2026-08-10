# Contributing to CanCan

Thanks for helping build a local-first financial evidence vault. This file describes how to set up a development machine, how work is organized in this repository, and what a pull request needs before it can merge.

## Development setup

From a fresh macOS checkout, run:

```bash
./scripts/setup-dev.sh
```

The script installs the pinned Node.js, Corepack, pnpm, and Rust toolchains into your home directory, bootstraps dependencies, and runs the verification gates. It does not require Homebrew or `sudo`. Use `./scripts/setup-dev.sh --dry-run` to inspect the plan without changing the machine.

After setup:

```bash
pnpm dev           # browser-hosted desktop UI during development
pnpm verify:fast   # file-size, typecheck, unit tests, web builds (fast gate)
pnpm verify        # full gate: adds Rust checks and the Tauri debug build
```

A pull request must pass the same harness as `main` before it can merge.

## How work is organized

- `docs/specs/` is the source of truth for intended product and implementation behavior. When behavior changes, the spec changes in the same pull request.
- `docs/agent/reading-order.md` gives the canonical reading order; `docs/agent/implementation-slices.md` splits the roadmap into reviewable slices.
- `.agents/scripts/agent-preflight.sh` validates the working tree and toolchain; run it before starting a task.

Before writing code, read the spec section that governs your change and the slice notes that apply. Do not implement through a slice's documented blockers.

## How work is tracked (issues)

GitHub issues are the single entry point for all work — fixes, features, refactors, docs. A feature request is an `enhancement`-category issue whose body proposes the spec change (a spec is never written before its issue exists); the issue's first PR lands the spec change and the implementation together. Categories, priority, and scope policy live in `.agents/workflows/issue-delivery.md`.

## Pull request requirements

- Keep pull requests small and reviewable; one concern per pull request.
- Every PR closes an issue: `Closes #<n>` in the description — except trivial no-behavior fixes (typo, comment, formatting, dead link), which may ship without an issue. Link the `docs/specs/` section the PR changes whenever behavior changes; a feature issue's first PR lands spec + implementation together.
- Include tests or other deterministic evidence for the change. Bug fixes should add a regression test where the project already has tests.
- Use conventional branch names (`feat/...`, `fix/...`, `chore/...`) and Conventional Commit subjects.
- Never include real financial data, account numbers, or secrets in tests, fixtures, screenshots, or pull request descriptions — synthetic fixtures only.

## Questions and discussion

Usage and how-to questions belong in [GitHub Discussions](https://github.com/lyuzizheng/cancan/discussions) (`Questions` category), not in issues. Early ideas and design proposals start in `Ideas`. Issues are reserved for reproducible bugs and implementation-ready accepted work.

Security reports are never public — see [SECURITY.md](SECURITY.md).
