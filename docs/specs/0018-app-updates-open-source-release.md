# 0018. App Updates, Forward Compatibility, and Open-Source Release Spec

## Goal

Define how CanCan ships frequent app and provider-parser updates through a classic fully open-source GitHub release flow without breaking local vaults or silently changing committed financial facts.

## Release provisioning checkpoint

The product identity and release policy below are accepted. Public promotion remains blocked until the domain, contacts, Cloudflare project, Apple signing/notarization identity, Tauri updater key and recovery copy, production Google project, dual-architecture evidence, and release credentials are actually provisioned and verified. Documentation must distinguish intended ownership from completed external setup.

Do not implement an unsigned updater, remote prompt/config download, or independently delivered parser package by inference.

## Stable decisions

- CanCan is a fully open-source application.
- The public identity is `CanCan`, the repository slug is `cancan`, and the canonical domain is `cancan.money`.
- `support@cancan.money` and `security@cancan.money` are the intended public contacts; they are not treated as live until provisioned and verified.
- Once provisioned, the founding owner controls the Cloudflare account/zone and privacy-policy publication; the founding owner already holds initial project-governance authority.
- The project uses the Apache License 2.0. The founding owner is the initial sole maintainer and release approver; additional maintainers require an explicit governance update.
- The public Git repository, version tags, source history, build workflow, and GitHub Releases are the canonical release record.
- GitHub Releases is the only binary/update artifact source. Direct download, the Tauri in-app updater, and an optional Homebrew Cask reuse one approved CI build/version and its release-owned installers, updater archives, signatures, and metadata.
- App versions use semantic versioning.
- A release is traceable from artifact to immutable source tag and CI run.
- MVP does not require a proprietary CanCan update backend.
- App/parser updates never silently rewrite committed ledger events.
- The public website is a static Cloudflare Pages project backed by the public GitHub repository. CanCan does not maintain a duplicate GitHub Pages site.
- GitHub owns source, releases, issue/PR workflow, discussions, and security reporting; the website presents the product, policy, help, and verified download links.
- Phase 1 supports macOS on both Apple Silicon (`arm64`) and Intel (`x86_64`).
- Windows desktop support is Phase 2 and does not block the Phase 1 macOS release.
- The first public release is a pre-1.0 preview, not a `1.0` stability promise.
- The first pre-1.0 preview includes local file ingestion, multiple user-authorized Gmail mailbox connections, Gmail attachment ingestion, and provider-approved transaction-email ingestion.
- `Automatically add high-confidence records` is available and on by default. A record auto-commits only when its structured AI confidence meets the package/document threshold and it passes every accepted deterministic hard gate in `0005`; review-only, uncalibrated, or experimental profiles remain in Review.
- The first-preview source target is DBS, HSBC, and UOB. DBS Bank and DBS Card are separate accounts/profiles under one DBS Money Source. HSBC and UOB do not need a fixed Bank/Card profile matrix and may initially expose Review-only profiles, but each advertised source needs at least one working package/profile with deterministic fixture/runtime evidence. CPF and additional sources are follow-ons rather than first-preview blockers. Package/document confidence thresholds and calibration evidence remain release blockers for any profile advertised as auto-commit eligible.
- Public Gmail availability is therefore a release prerequisite. The production consent screen, website disclosures, privacy/support contacts, restricted-scope justification, and required Google verification must complete before the preview ships.

## Platform scope

Phase 1 has one macOS product contract across both architectures:

```text
macOS arm64
macOS x86_64
```

Both architectures must pass the same Vault, SQLCipher, Keychain, sidecar, parser, backup/restore, signing/notarization, updater, and user-flow gates before they are advertised as supported. Current end-to-end evidence exists only for `arm64`; deterministic Intel setup routing is not a substitute for a real `x86_64` build and runtime pass.

The release pipeline may later choose separate architecture artifacts or one universal macOS artifact. That packaging choice is not part of the platform-support decision and must be validated before release rather than inferred here.

The minimum supported version is macOS 14 Sonoma for both architectures. CI runner selection and packaging metadata must enforce the same floor rather than inheriting the current developer machine.

Windows is a Phase 2 port. Phase 2 must separately define supported Windows versions and architectures, installer/update format, code signing, OS secret storage, filesystem semantics, and equivalent Vault/security tests. Do not add Windows-specific production branches, dependencies, CI, or release claims during Phase 1 unless a focused Phase 2 slice is explicitly started.

A native macOS Finder `Share > CanCan` intake entry is also Phase 2. When Desktop is closed or locked, the selected files use bounded App Group handoff staging until import after unlock. Its dedicated slice owns the extension/service target, entitlements, staged-plaintext protection/cleanup/expiry, supported multi-file behavior, signing/notarization, and packaged runtime evidence. It reuses the existing host Add/capture path and must not create a second parser, Vault owner, source registry, or durable job queue.

## Future iOS intake companion

The accepted mobile direction is a thin native Swift containing app plus Share Extension whose only product responsibility is evidence intake. It is not part of Phase 1, does not contain the ledger or desktop Vault, and must not delay the desktop evidence/reconciliation loop.

Before committing the production transport architecture, run a disposable local Xcode feasibility slice covering:

```text
Share-sheet PDF/CSV/image acceptance and extension lifecycle limits
App Group handoff between the extension and containing app
candidate cross-device transport and retry while the desktop is offline
encryption before transport without giving the phone the desktop Vault master key
duplicate/replay behavior, deletion, pairing/recovery, and truthful completion UX
native target ownership outside generated Tauri desktop artifacts
```

That spike chooses or rejects the transport boundary; it does not create public app identity, production signing, TestFlight, or App Store Connect state. A later explicitly authorized mobile release slice must separately prove Apple Developer ownership, production signing/provisioning, required privacy disclosures, TestFlight distribution, App Store review assets, supported iOS versions/devices, and release/revocation operations before the companion ships.

Until that slice is accepted, the supported phone workflow is Save to Files/AirDrop/email into the desktop ingestion channels. Do not create a mobile account service, hosted upload backend, or multi-device ledger sync by implication.

Primary references:

- [Apple App Extension Programming Guide](https://developer.apple.com/library/archive/documentation/General/Conceptual/ExtensibilityPG/index.html)
- [Apple Share extension guide](https://developer.apple.com/library/archive/documentation/General/Conceptual/ExtensibilityPG/Share.html)
- [Apple App Groups entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.application-groups)

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
local-first/privacy explanation for the capabilities that actually ship
security model, vulnerability-reporting route, and supported-version policy
documentation/help for the capabilities that actually ship
community/contributing links
```

Before the pre-1.0 preview ships, add the Gmail connection explanation, Google API Services User Data Policy Limited Use disclosure, verified privacy/support contacts, and every public OAuth requirement owned by `0003-gmail-collector.md`. Website and in-app copy must distinguish attachment ingestion from separately consented provider-approved transaction-email body ingestion.

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
- The founding owner sets and documents the initial triage cadence and service expectations; adding another maintainer requires an explicit governance update.

These files are normative under Apache-2.0 and the founding-owner governance defined above. Use GitHub's native templates rather than maintaining a second intake system.

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

For Phase 1, release evidence must identify whether each artifact targets macOS `arm64`, macOS `x86_64`, or a validated universal binary. Passing on one architecture does not qualify the other.

Publish binaries from CI, not an unrecorded developer-machine build. A platform artifact must not be advertised as supported when its required signing or compatibility gate did not pass. Published tags and artifacts follow the accepted immutable correction/revocation policy; enabling GitHub's optional immutable-release protection is a provisioning choice only and must preserve that policy. Public repositories can add artifact attestations and SBOM attestations to improve supply-chain verification.

The first `0.x Preview` versions are ordinary published GitHub Releases with honest Preview product copy, not a separate GitHub prerelease channel. This keeps one release selector and lets the updater resolve the approved release through GitHub's `releases/latest` asset path. A draft release is never update-visible.

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

Keep one `0.x Preview` channel. Setup exposes one update-check switch, off by default; manual checking is always available. Do not add alpha/beta channel routing. Do not force an update silently. A security-critical update receives a prominent explanation and still offers `Install now` or `Remind later`.

When automatic checks are enabled, check at most once per successful local day and do not show UI when no update exists. Manual checking bypasses that cadence. The update sheet shows the current and target version, release notes, known download size, restart requirement, and compatibility/migration warning before `Install and restart` or `Remind later`.

ADR 0001 is accepted, so implementation uses the Tauri signed updater with GitHub-hosted updater artifacts and `latest.json`. CI generates the platform updater archives, signatures, and `latest.json` from the same approved app build after signing/notarization; they remain on the draft release until human approval publishes that release. The app embeds the updater public key and uses the static GitHub `releases/latest/download/latest.json` endpoint; it does not query mutable remote config or a CanCan update server.

Apple Developer ID signing/notarization and the Tauri updater signature are separate required checks. Apple establishes the distributed app bundle's platform identity; the updater signature authorizes the archive to an installed CanCan client. HTTPS, a GitHub Release, or a Homebrew SHA-256 does not replace either check.

The founding owner is the initial sole human release approver. Once provisioned, the owner must hold the Apple Developer signing identity; the Tauri updater private key and password must live in protected CI release secrets with one encrypted offline recovery copy controlled by the owner. Published source tags and release artifacts are immutable; a correction ships as a new version, while a security incident may revoke or withdraw an affected artifact with a public explanation.

Reference: [Tauri updater signing and GitHub release metadata](https://v2.tauri.app/plugin/updater/).

## Homebrew Cask distribution

Homebrew is an optional installation and upgrade entry, not CanCan's update authority or a second release pipeline.

Initial distribution may use a project-owned tap. A later official `homebrew-cask` submission is a discoverability improvement and does not change the artifact or trust model.

The Cask must:

```text
use a concrete semantic version, never version :latest
reference the matching versioned macOS artifact from GitHub Releases
pin its published SHA-256
declare auto_updates true because CanCan provides a real in-app installer
install the same bundle identifier and version used by direct download and the updater
```

Users may update through CanCan or `brew upgrade --cask`; both paths converge on the same signed/notarized app version. The release workflow updates the Cask only after the corresponding GitHub Release is public. Homebrew metadata, tap automation, or checksum verification must never publish, rebuild, resign, or substitute a different application artifact.

References:

- [Homebrew Cask Cookbook](https://docs.brew.sh/Cask-Cookbook)
- [Homebrew self-updating Cask behavior](https://docs.brew.sh/FAQ)
- [Homebrew project-owned taps](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)

## Provider-parser delivery

Provider parser packages, prompts, schemas, validators, and confidence-calibration metadata change frequently and carry explicit versions.

Safe MVP boundary:

```text
ship parser updates inside signed app releases
do not download unsigned prompts, parser logic, or confidence-calibration config at runtime
preserve the exact parser/prompt/schema/validator versions used by every parse run
new parser versions start uncalibrated and follow the accepted calibration + hard-gate evidence
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

- targets Apple Silicon and Intel macOS; Apple Silicon is verified end to end, while Intel setup routing is deterministic-test-covered and still requires a real Intel build/runtime pass before support can ship;
- requests Apple Command Line Tools when missing, then asks the developer to rerun after Apple's installer finishes;
- installs the pinned official Node.js binary under the user's home directory and verifies its published SHA-256 checksum;
- refuses to replace regular files, directories, or foreign symlinks already occupying its `~/.local/bin` tool paths;
- when an existing `fnm` installation controls the interactive shell, installs the same Node/Corepack/pnpm pins there without changing the user's default Node version;
- installs pinned Corepack and pnpm, then installs the pinned Rust toolchain with clippy/rustfmt through rustup;
- uses project-local Tauri CLI dependencies rather than a global Tauri installation;
- adds only `~/.local/bin` and `~/.cargo/bin` to the shell profile, idempotently;
- bootstraps the production workspace with its frozen lockfile and runs preflight, the production application gate, and the desktop feasibility gate by default;
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

The development-only Tauri identifier is `dev.cancan.desktop`, the app version is `0.0.0`, bundling is disabled, and the generated icon is provisional. These values exist only to make the local debug build real. The public identifier uses the reversed `cancan.money` namespace with the `cancan` app slug; the exact packaged identifier is frozen with signing/notarization setup rather than by the development identifier.

References:

- [Node.js 24.18.0 LTS release](https://nodejs.org/en/blog/release/v24.18.0)
- [pnpm installation and Corepack pinning](https://pnpm.io/installation#using-corepack)
- [Corepack 0.35.0 release](https://github.com/nodejs/corepack/releases/tag/v0.35.0)
- [Rust 1.97.0 release](https://blog.rust-lang.org/releases/latest/)
- [official rustup installer](https://rustup.rs/)
- [Tauri macOS prerequisites](https://v2.tauri.app/start/prerequisites/#macos)

Before the corresponding public capability or release surface goes live, a maintainer must deliberately provision and record ownership/recovery for:

```text
GitHub repository, Actions, Releases, Discussions, private vulnerability reporting, environments, and release approvers
Cloudflare account/zone, custom domain, Pages project, and least-privilege repository integration
Google Cloud development and production projects, Gmail API, OAuth consent screen/client, test users, Search Console domain ownership, and public support contact before public Gmail availability
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
- GitHub Releases is the single artifact source for direct download, the Tauri updater, and the optional Homebrew Cask.
- One ordinary `0.x Preview` GitHub Release supplies the signed `latest.json` metadata; CanCan does not add alpha/beta routing or a second update service.
- Homebrew uses a concrete version and checksum for the same release artifact, declares the real in-app updater, and never owns a separate build or signing path.
- macOS platform signing/notarization and Tauri updater signing both pass; GitHub hosting and Homebrew checksums do not replace either boundary.
- Parser updates are versioned, confidence-calibrated independently, and cannot silently rewrite committed facts.
- MVP does not download unsigned parser/prompt/config updates.
- Older incompatible apps refuse mutation and explain the required version.
- Tauri updater work follows the accepted single-channel, owner-approved key-custody policy and remains gated on actual credential provisioning and recovery proof.
- Static public pages deploy to preview and production environments without adding an application backend or analytics.
- GitHub community files route support, bugs, contributions, and private security reports without asking users to expose financial data.
- A release is not promoted until its public website/privacy claims, supported platforms, license, signing, updater-key recovery, and approval owner are complete.
- The first public release uses a pre-1.0 version and honest preview language; review-only providers remain in Review and do not become stability or auto-add claims.
- The first pre-1.0 release includes local files, multiple Gmail mailbox connections, Gmail attachments, and provider-approved transaction-email ingestion only after its public OAuth, consent, disclosure, and verification gates pass.
- The default-on auto-commit toggle applies only when structured AI confidence meets the accepted package/document threshold and every accepted deterministic hard gate passes; all other records remain in Review.
- A fresh supported development machine can install and verify the pinned toolchain with one repository command; setup version drift and non-idempotent profile edits fail deterministic tests.
- Phase 1 release evidence covers both macOS `arm64` and `x86_64`; evidence from one architecture never qualifies the other.
- Windows remains Phase 2 and creates no Phase 1 implementation or release requirement.
- The future iOS Share Extension remains a separate thin intake companion: a disposable technical spike selects its transport boundary, and a later authorized release slice owns production signing, TestFlight, and App Store evidence.
