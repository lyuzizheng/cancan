# Security Policy

CanCan handles financial evidence, so security reports are handled privately and deliberately. **Never report a vulnerability in a public issue, discussion, or pull request.**

## Reporting a vulnerability

Use one of these private routes:

1. **GitHub private vulnerability reporting** — once the repository is public and the feature is enabled by the owner, report through the repository's *Security → Advisories → Report a vulnerability* flow. This is the preferred route: it lets us coordinate a fix and disclosure without exposing details.
2. **Email** — `security@cancan.money` is intended as the private security contact and is being provisioned; it will be verified before the first public preview ships. Until that address is confirmed live, use the GitHub route above or contact the maintainer directly through their GitHub profile at https://github.com/lyuzizheng.

When reporting:

- Include redacted reproduction steps, affected version, and operating system/architecture.
- **Never include real financial data, account numbers, statement files, or secrets** in a report — synthetic examples are enough.
- Disclosure timing is coordinated with you. The founding owner sets and documents the project's initial triage cadence and service expectations separately; until they are published here, treat response times as best-effort.

## Supported versions

CanCan is in a pre-1.0 preview line. Only the latest published `0.x` preview release receives security fixes; fixes ship as new releases, never as silent patches. A security-critical update is explained prominently in the app and still waits for your approval to install.

## Security model summary

- The Vault is a SQLCipher-encrypted database and encrypted file store on your Mac, unlocked by your password with an Argon2id-derived key.
- Vault keys, remembered unlock, statement passwords, and OAuth refresh tokens live in the macOS Keychain.
- Document viewing renders in memory — no plaintext temp files.
- The renderer never receives raw file bytes, filesystem paths, hashes, locators, or database handles.
- Network access exists only for connections you enable: Gmail (official API, read-only scope), your own AI provider, and optional update checks that verify signed metadata.
- Releases are CI-built from immutable tags with checksums, signatures, and provenance — never developer-machine builds.

For the full model, see the security page on the project website and `docs/specs/` in this repository.
