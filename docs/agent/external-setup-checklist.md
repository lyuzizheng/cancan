# External Setup Checklist

Last verified: 2026-07-28

This is the current evidence tracker for owner-controlled accounts, credentials,
contacts, and public-release provisioning. It does not redefine product behavior
or authorize implementation through a blocked slice.

Canonical behavior remains in:

- [Gmail Collector](../specs/0003-gmail-collector.md)
- [Testing, Fixtures, and Agent Gates](../specs/0016-testing-fixtures-agent-gates.md)
- [App Updates and Open-Source Release](../specs/0018-app-updates-open-source-release.md)

Status terms:

- `verified`: evidence exists and the owner has confirmed control or recovery.
- `partial`: some evidence exists, but launch-critical control or recovery is
  incomplete.
- `not verified`: no current repository evidence proves setup.
- `blocked`: another named checkpoint must complete first.

## Current evidence

| Area | Status | Current evidence | Next owner action |
| --- | --- | --- | --- |
| Domain | partial | The owner reports that `cancan.money` is purchased. DNS observation on 2026-07-28 returned Cloudflare nameservers `brodie.ns.cloudflare.com` and `emily.ns.cloudflare.com`. Account ownership, renewal, registrar lock, and recovery are not recorded. | Confirm the zone is in the intended Cloudflare account, enable two-factor authentication, and record renewal/recovery ownership outside Git. |
| Public contacts | not verified | `support@cancan.money` and `security@cancan.money` are intended but not evidenced as receiving or reply-capable. | Choose the intended destination mailbox and reply identity now; provision and test both addresses only after `public-project-surface` is authorized. |
| Cloudflare Pages | blocked | No Pages project or preview URL is recorded. | Wait for the website's local static build, then authorize the GitHub integration for only this repository and create a preview deployment before binding the custom domain. |
| Apple Developer | not verified | No paid membership, release identity, Developer ID certificate, Team ID, or notarization credential is recorded. | Choose individual or organization enrollment and collect its prerequisites now. Wait for `backup-release` authorization before paid enrollment, and do not create the release certificate until the production bundle identifier is frozen. |
| Google development OAuth | not verified | No development/test Google Cloud project or Desktop OAuth client is recorded. | Choose the owning Google account, development project name, and test mailboxes now. Wait for `gmail-onboarding` authorization before creating the project/client; request only `gmail.readonly`. |
| Google production OAuth | blocked | No production project, Search Console evidence, production client, or verification submission is recorded. | Verify `cancan.money` in Search Console now; wait for the running Gmail flow and final disclosures before creating the verification package and submitting the production project. |
| GitHub public project | partial | `lyuzizheng/cancan` is private. Issues and Discussions are enabled. Required public community-health files and private vulnerability reporting are not evidenced. Hosted jobs currently fail to start under the private-repository billing condition. | Complete the public-history/secret audit and community files before the owner changes visibility. Enable private vulnerability reporting after the repository is public. |
| macOS release identity | blocked | The development config remains `dev.cancan.desktop`, version `0.0.0`, bundling disabled, and a provisional icon. | Freeze the public bundle identifier and icon during release setup, then configure and prove signing, notarization, stapling, and Gatekeeper behavior. |
| Tauri updater | blocked | No updater key, Actions secret, or offline recovery proof is recorded. | Generate the password-protected updater key only after the release slice is authorized; store the private key in Actions and an owner-controlled offline recovery location. |
| Dual-architecture release | blocked | Real end-to-end evidence exists only for `arm64`. | Run the same build, Vault, Keychain, sidecar, parser, backup/restore, signing, updater, and user-flow gates on a real `x86_64` environment before advertising Intel support. |

No row becomes `verified` from planned setup, screenshots, documentation, a
merged pull request, or a command that did not actually execute.

## Owner checklist to start now

- [ ] Confirm control, two-factor authentication, recovery, registrar lock, and
  renewal for `cancan.money`.
- [ ] Choose the destination mailbox and reply identity that
  `support@cancan.money` and `security@cancan.money` will use. Do not provision
  them before `public-project-surface` is authorized.
- [ ] Choose Apple Developer enrollment:
  - Individual is the direct path when no organization legal entity exists.
  - Organization enrollment requires the intended legal entity and D-U-N-S
    evidence.
- [ ] Record the expected Apple enrollment cost without paying yet. The current
  official price is USD 99 per membership year or the available local-currency
  equivalent; verify the displayed amount when `backup-release` authorizes
  enrollment.
- [ ] Choose the owning Google account, intended development-project name, and
  test mailboxes without creating the Google Cloud project or OAuth client yet.
- [ ] Add `cancan.money` as a Google Search Console domain property and complete
  its DNS ownership challenge.
- [ ] Choose the owner-controlled password-manager and offline-recovery
  locations for signing credentials, updater keys, and account recovery codes.

Do not put OAuth tokens, a mistakenly issued desktop client secret, Apple
credentials, updater private keys, provider API keys, recovery codes, or
passwords in Git, fixtures, logs, pull requests, or chat transcripts.

## Hold until the named checkpoint

| Setup | Earliest checkpoint |
| --- | --- |
| `support@cancan.money` and `security@cancan.money` provisioning | `public-project-surface` is authorized |
| Google development project, Gmail API, Desktop OAuth client, and test users | `gmail-onboarding` is authorized |
| Paid Apple Developer Program enrollment | `backup-release` is authorized |
| Cloudflare Pages project and repository authorization | Website local build and link/accessibility checks pass |
| `cancan.money` production binding | Cloudflare PR preview is accepted |
| Production Google OAuth project and submission | The real multi-mailbox flow, public homepage/privacy copy, provider-bound AI disclosure, and demo video are reviewable |
| Developer ID certificate and notarization CI secret | Production bundle identifier and release workflow are frozen |
| Tauri updater private key | `backup-release` is authorized and an owner-controlled recovery location is ready |
| Repository visibility change | Full-history secret/public-data audit and community-health files pass review |
| Public download links | Signed and notarized immutable GitHub Release artifacts exist |

The production Gmail client uses
`https://www.googleapis.com/auth/gmail.readonly`, which Google classifies as a
Restricted scope. Google makes the final verification and security-assessment
determination. The local-only architecture and lack of a CanCan Gmail backend
must be disclosed together with any direct, separately consented BYO-AI
transfer; neither the repository nor the owner should claim an assessment
exception in advance.

## Safe work while the website writer is active

One customer-facing source writer remains active on the website branch. Safe
parallel work is therefore limited to:

- the owner-controlled account, domain, contact, and recovery tasks above;
- read-only repository history, secret-exposure, release-readiness, and CI
  audits;
- local screenshot capture and E2E/release test planning that do not modify the
  website branch;
- independent review of a frozen website checkpoint.

Do not start a second production-code implementation lane, including Gmail,
auto-commit, backup/release, or unrelated renderer work, until the active website
writer reaches a reviewable checkpoint or stops.

## Responsibility boundary

The coding agent can:

- implement repository code, static-site build configuration, OAuth and release
  integration, community files, CI, updater wiring, and tests;
- draft privacy, security, support, Google verification, and demo materials for
  owner approval;
- run public-history and secret scans without printing discovered secrets;
- generate a CSR or password-protected updater key after explicit authorization,
  configure secret names, and verify signing/notarization with credentials
  supplied through the intended secret store.

The owner must personally complete or approve:

- payments, legal identity, program agreements, identity checks, D-U-N-S, two
  factor authentication, and account-recovery enrollment;
- final domain, Cloudflare, Apple, Google, GitHub, public-contact, and privacy
  ownership;
- custody and recovery of Apple and updater private keys;
- Google verification declarations and responses to review;
- the repository visibility change and final public release approval.

Google controls OAuth approval and whether a third-party security assessment is
required. An agent cannot guarantee either outcome or schedule.

## Cost and timing watch

- Apple Developer Program is the known planned paid prerequisite for direct
  Developer ID distribution.
- Cloudflare static Pages assets and inbound Email Routing can use the free
  service limits for this static design; do not add Workers or Pages Functions.
- Standard GitHub-hosted runners are currently free and unlimited for public
  repositories. This may remove the present private-repository Actions billing
  no-start after the repository safely becomes public; it is not a reason to
  skip the pre-public audit.
- Google Restricted Scope verification can take several weeks. A third-party
  security assessment may add material cost and recurring work if Google
  requires one.

Recheck external prices, limits, and review instructions at the time of
enrollment or submission.

## Official setup references

- [Apple Developer membership](https://developer.apple.com/support/compare-memberships/)
- [Apple Developer ID certificates](https://developer.apple.com/help/account/certificates/create-developer-id-certificates/)
- [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- [Cloudflare Pages GitHub integration](https://developers.cloudflare.com/pages/configuration/git-integration/github-integration/)
- [Cloudflare Pages custom domains](https://developers.cloudflare.com/pages/configuration/custom-domains/)
- [Cloudflare Email Routing](https://developers.cloudflare.com/email-service/get-started/route-emails/)
- [Google Gmail API scopes](https://developers.google.com/workspace/gmail/api/auth/scopes)
- [Google Restricted Scope verification](https://developers.google.com/identity/protocols/oauth2/production-readiness/restricted-scope-verification)
- [GitHub repository visibility](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/managing-repository-settings/setting-repository-visibility)
- [GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
