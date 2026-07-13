# cancan

All money source can.

CanCan is a local-first financial evidence vault and reconciliation console. It collects financial evidence from Gmail, PDFs, CSVs, exports, APIs, and manual uploads; extracts and normalizes records with AI-assisted parsing; validates results deterministically; links related money movements across sources; and keeps every committed record traceable to source evidence.

Start with [docs/README.md](docs/README.md).

## Development

The currently verified development target is macOS. From a fresh checkout, run:

```bash
./scripts/setup-dev.sh
```

The script installs the pinned Node.js, Corepack, pnpm, and Rust toolchains into your home directory, bootstraps dependencies, and runs both the production application gate and the desktop feasibility gate. Apple Silicon is verified end to end; Intel setup routing is test-covered but still awaits a real Intel hardware run. The script does not require Homebrew or `sudo`. If macOS opens the Command Line Tools installer, finish it and rerun the same command.

Toolchain pins and their verification date live in [`scripts/dev-toolchain.env`](scripts/dev-toolchain.env). Use `./scripts/setup-dev.sh --dry-run` to inspect the plan without changing the machine.

After setup:

```bash
pnpm dev       # browser-hosted desktop UI during development
pnpm verify    # typecheck, unit tests, Rust checks, and Tauri debug build
```
