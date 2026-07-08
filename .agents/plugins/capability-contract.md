# Capability Contract

Agents may use available tools, plugins, and connectors only within CanCan's safety model.

## Allowed

- Read repository files.
- Run local tests and scripts.
- Use browser/Chrome/Playwright/computer-use for UI inspection.
- Use source connectors in mocked or explicitly authorized local flows.

## Not Allowed

- Mutate Gmail or external financial accounts.
- Bypass MFA or CAPTCHAs.
- Send secrets to AI providers.
- Store secrets in plain SQLite.
- Claim provider support without fixtures and tests.

## Selection Rule

Prefer deterministic scripts and repo-local commands when available. Use external tools only when they add verification that local commands cannot provide.
