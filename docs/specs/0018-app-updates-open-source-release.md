# 0018. App Updates, Forward Compatibility, and Open-Source Release Spec

## Goal

Define how CanCan ships frequent app and provider-parser updates through a classic fully open-source GitHub release flow without breaking local vaults or silently changing committed financial facts.

## Implementation blocker

MVP operating systems, project license, public identity/domain/contacts, signing/notarization identities, updater-key custody, update channels, release approval policy, and whether parser packages may ever update independently remain unresolved in the [active alignment register](../alignment-temp/alignment-progress.md).

Do not implement an unsigned updater, remote prompt/config download, or platform-specific release pipeline by inference.

## Stable decisions

- CanCan is a fully open-source application.
- The public Git repository, version tags, source history, build workflow, and GitHub Releases are the canonical release record.
- App versions use semantic versioning.
- A release is traceable from artifact to immutable source tag and CI run.
- MVP does not require a proprietary CanCan update backend.
- App/parser updates never silently rewrite committed ledger events.
- The public website is a static Cloudflare Pages project backed by the public GitHub repository. CanCan does not maintain a duplicate GitHub Pages site.
- GitHub owns source, releases, issue/PR workflow, discussions, and security reporting; the website presents the product, policy, help, and verified download links.

## Public website and project surfaces

The static site should deploy through Cloudflare Pages Git integration:

```text
pull request -> preview deployment for review
main branch -> production deployment
custom domain -> public canonical site
```

First public pages:

```text
landing and product story
downloads and release notes linking to verified GitHub Releases artifacts
local-first/privacy explanation and Google API Services User Data Policy Limited Use disclosure
security model, vulnerability-reporting route, and supported-version policy
documentation/help and Gmail connection explanation
community/contributing links
```

Keep the site static. Do not add Cloudflare Workers/Pages Functions, hosted accounts, behavioral analytics, trackers, or a second product backend for the landing page. Any future telemetry requires a separate explicit product/privacy decision.

Cloudflare Pages provides GitHub push deploys, pull-request preview deployments, and custom-domain support. The domain, Cloudflare account/zone owner, copy, privacy-policy owner, and public support/security contacts remain launch blockers.

References:

- [Cloudflare Pages GitHub integration](https://developers.cloudflare.com/pages/configuration/git-integration/github-integration/)
- [Cloudflare Pages preview deployments](https://developers.cloudflare.com/pages/configuration/preview-deployments/)
- [Cloudflare Pages custom domains](https://developers.cloudflare.com/pages/configuration/custom-domains/)

## GitHub community operating model

Before the repository is promoted publicly, add and maintain:

```text
LICENSE
CONTRIBUTING.md
CODE_OF_CONDUCT.md
SECURITY.md with private vulnerability-reporting instructions
SUPPORT.md
.github issue forms for reproducible bugs and feature proposals
.github pull request template
GitHub Discussions categories for Questions, Ideas, and Show and tell
```

Route work deliberately:

- Discussions `Questions` is the first stop for usage/support questions; `Ideas` is for early proposals and community design discussion.
- Issues are for reproducible bugs and implementation-ready accepted work. Templates collect version, operating system, reproduction, expected/actual behavior, and redacted diagnostics without financial data or secrets.
- Security reports use GitHub private vulnerability reporting or the private route in `SECURITY.md`, never a public issue.
- Pull requests should be small, linked to an issue/spec when behavior changes, include tests/evidence, and pass the same harness as `main`.
- Maintainers triage labels and unanswered Discussions on a documented cadence; exact ownership and service expectations wait for named maintainers.

The project license and governance authority must be chosen before these files become normative. Use GitHub's native templates rather than maintaining a second intake system.

Reference: [GitHub issue and pull request templates](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/about-issue-and-pull-request-templates).

## GitHub release pipeline

Target flow:

```text
version/changelog pull request
-> reviewed commit on main
-> immutable semantic-version source tag
-> GitHub Actions deterministic tests and harness gates
-> platform builds
-> platform signing/notarization where required
-> checksums, signatures, update metadata, SBOM/dependency-license report, and artifact provenance attestation
-> draft GitHub Release
-> human release approval
-> publish the approved GitHub Release and release notes
-> update website download metadata from the verified release
```

Release artifacts should include, where applicable:

```text
source tag/archive
human-readable changelog
platform installer/package
SHA-256 checksums
cryptographic signatures and signed update metadata
dependency/license or SBOM-style report
artifact provenance attestation
migration/compatibility notes
```

Publish binaries from CI, not an unrecorded developer-machine build. A platform artifact must not be advertised as supported when its required signing or compatibility gate did not pass. Whether to enable GitHub's immutable release protection is part of the unresolved release policy; use it only if it fits the finalized approval, correction, and revocation workflow. Public repositories can add artifact attestations and SBOM attestations to improve supply-chain verification.

References:

- [Tauri distribution overview](https://v2.tauri.app/distribute/)
- [Tauri GitHub Actions pipeline](https://v2.tauri.app/distribute/pipelines/github/)
- [Tauri macOS signing and notarization](https://v2.tauri.app/distribute/sign/macos/)
- [GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations)
- [GitHub supply-chain security](https://docs.github.com/en/code-security/concepts/supply-chain-security/supply-chain-security)

## Incremental app update behavior

The app may check release metadata backed by GitHub Releases only when the user enables update checks or invokes a manual check. It must verify signed update metadata and artifact signatures before installation.

The normal UX should show:

```text
current and available version
release notes
download size when known
whether restart is required
compatibility or migration warning
install now / remind later
```

Do not force an update silently. Security-critical update policy and stable/beta channels remain unresolved.

ADR 0001 is accepted, so implementation may use the Tauri signed updater with GitHub-hosted artifacts and `latest.json`. The updater signing public key may be embedded in the app; the private key and password must remain release secrets. Back up the private key through a documented offline recovery path because losing it prevents signed updates to installed clients.

Reference: [Tauri updater signing and GitHub release metadata](https://v2.tauri.app/plugin/updater/).

## Provider-parser delivery

Provider parser packages, prompts, schemas, validators, and qualification metadata change frequently and carry explicit versions.

Safe MVP boundary:

```text
ship parser updates inside signed app releases
do not download unsigned prompts, parser logic, or qualification config at runtime
preserve the exact parser/prompt/schema/validator versions used by every parse run
new parser versions start unqualified and follow fixture + shadow gates
installing an app update does not automatically reparse or mutate committed facts
```

A later independently signed parser-pack channel requires a separate accepted design for package signatures, compatibility, rollback, revocation, and update authority.

## Forward compatibility

[0009 Backup, Restore, and Versioning](0009-backup-restore-versioning.md) owns vault/schema/backup compatibility. App updates must obey its manifest, minimum-reader, pre-migration backup, and no-unsafe-downgrade rules.

Additional parser compatibility rules:

- old parse runs remain readable with their recorded package versions;
- new parsers produce new versions rather than overwriting previous outputs;
- optional reparse creates proposals/review work and never silently changes committed ledger facts;
- removed provider support must retain historical rendering and evidence access;
- unsupported newer data causes a clear upgrade-required/read-only failure, not best-effort mutation.

## Rollback

Keep prior public release artifacts available unless a security incident requires revocation.

Application rollback does not imply data rollback. An older app must refuse write access when the current vault/schema/parser metadata requires a newer reader. Data rollback uses verified backup/restore behavior owned by [0009](0009-backup-restore-versioning.md).

## Developer and maintainer preparation

The app-foundation slice provides one real macOS command without requiring public launch accounts:

```text
./scripts/setup-dev.sh
```

The setup command:

- supports Apple Silicon macOS, verified end to end, and has deterministic setup coverage for Intel macOS pending a real Intel hardware run;
- requests Apple Command Line Tools when missing, then asks the developer to rerun after Apple's installer finishes;
- installs the pinned official Node.js binary under the user's home directory and verifies its published SHA-256 checksum;
- refuses to replace regular files, directories, or foreign symlinks already occupying its `~/.local/bin` tool paths;
- when an existing `fnm` installation controls the interactive shell, installs the same Node/Corepack/pnpm pins there without changing the user's default Node version;
- installs pinned Corepack and pnpm, then installs the pinned Rust toolchain with clippy/rustfmt through rustup;
- uses project-local Tauri CLI dependencies rather than a global Tauri installation;
- adds only `~/.local/bin` and `~/.cargo/bin` to the shell profile, idempotently;
- bootstraps the current package root with its frozen lockfile and runs preflight plus the desktop feasibility gate by default;
- requires neither Homebrew nor `sudo`.

`scripts/dev-toolchain.env` is the setup version source. `.node-version`, `rust-toolchain.toml`, and package-manager/Tauri pins must match it, and `scripts/test-setup-dev.sh` enforces that invariant. Versions are intentionally pinned for reproducibility rather than floating to an unreviewed future `latest` on each machine.

Latest supported stable pins verified against official upstreams on 2026-07-13. Node uses the latest LTS line rather than the short-lived Current line; the remaining tools use their latest stable release:

```text
Node.js 24.18.0 LTS
Corepack 0.35.0
pnpm 11.12.0
Rust 1.97.0 stable
Tauri CLI 2.11.4 (project-local)
```

When upgrading, verify the new upstream releases, update every pin in one patch, and run the setup simulation, a real repeat setup, application gates, preflight, harness self-test, and independent semantic review.

References:

- [Node.js 24.18.0 LTS release](https://nodejs.org/en/blog/release/v24.18.0)
- [pnpm installation and Corepack pinning](https://pnpm.io/installation#using-corepack)
- [Corepack 0.35.0 release](https://github.com/nodejs/corepack/releases/tag/v0.35.0)
- [Rust 1.97.0 release](https://blog.rust-lang.org/releases/latest/)
- [official rustup installer](https://rustup.rs/)
- [Tauri macOS prerequisites](https://v2.tauri.app/start/prerequisites/#macos)

Before public release, a maintainer must deliberately provision and record ownership/recovery for:

```text
GitHub repository, Actions, Releases, Discussions, private vulnerability reporting, environments, and release approvers
Cloudflare account/zone, custom domain, Pages project, and least-privilege repository integration
Google Cloud development and production projects, Gmail API, OAuth consent screen/client, test users, Search Console domain ownership, and public support contact
Apple Developer Program signing/notarization credentials if macOS is distributed outside the App Store
platform code-signing credentials for every other supported operating system
Tauri updater signing key, Actions secret, and offline recovery copy
public privacy, support, and security contacts
```

BYO-AI provider keys belong only in the user's local OS secret store. They are not project deployment credentials and must not be added to CI. Account creation, domain purchase, OAuth submission, paid signing enrollment, and secret generation happen only when their slice is authorized; documentation must never imply that they already exist.

## Acceptance criteria

- Every published app artifact traces to a public immutable source tag and GitHub Actions run.
- Releases include checksums, signatures where required, notes, and compatibility information.
- Update artifacts are signature-verified and are not silently forced.
- Parser updates are versioned, qualified independently, and cannot silently rewrite committed facts.
- MVP does not download unsigned parser/prompt/config updates.
- Older incompatible apps refuse mutation and explain the required version.
- Tauri updater work remains gated on finalized signing-key custody, release channels, supported operating systems, and release approval.
- Static public pages deploy to preview and production environments without adding an application backend or analytics.
- GitHub community files route support, bugs, contributions, and private security reports without asking users to expose financial data.
- A release is not promoted until its public website/privacy claims, supported platforms, license, signing, updater-key recovery, and approval owner are complete.
- A fresh supported development machine can install and verify the pinned toolchain with one repository command; setup version drift and non-idempotent profile edits fail deterministic tests.
