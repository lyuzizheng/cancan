# 0007. First-Run and Onboarding Spec

## Goal

Define the first-launch experience and startup sequence for CanCan.

The first-run experience must communicate technology, safety, and local ownership while staying practical.

## Implementation blocker

First-run optionality and exact Gmail/cloud-AI data consent remain unresolved in the [active alignment register](../alignment-temp/alignment-progress.md). The simple password/Keychain/recovery-file model is accepted; exact cryptographic claims still depend on validated implementation evidence.

## Experience goal

Onboarding is a designed product experience, not a settings form shown before the app.

It should feel:

```text
beautiful and future-facing
calm, secure, and local
progressive rather than configuration-heavy
animated with clear state meaning
transparent about which capabilities are on or off
```

Use a focused full-screen flow with one primary decision per step, strong typography, precise motion, clear progress, and a persistent sense that the user is building a private local vault.

## Opening message

The opening screen should feel modern, warm, and secure.

Draft positioning copy:

```text
CanCan is your personal local money can.
Bring your own AI, add statements, emails, and exports from the channels you choose, and turn them into a private financial vault you control.
```

Copy should be refined during UI design, but the core ideas must remain:

- personal;
- local-first;
- money-source centric;
- bring-your-own AI;
- secure vault;
- automated evidence collection.

Avoid defensive product copy that explains what CanCan does not do. UI copy should feel simple, confident, and useful.

## Provider policy

CanCan should not let users create arbitrary integration providers for MVP.

MVP providers are fixed and product-defined:

```text
DBS bank account statement
DBS credit card statement
UOB bank account statement
UOB credit card statement
Wise PDF/CSV/export
```

Users can create accounts/sub-accounts under supported money sources. Custom/manual assets can exist later, but they must not be confused with supported automated integrations.

## First-run flow

Recommended flow:

```text
Welcome / product promise
-> How local-first works
-> How the vault protects data
-> Create local vault
-> Choose vault password / local key setup
-> Save the recovery file now, or choose `Skip for now` and continue with recovery visibly not configured
-> AI provider setup
-> Add a first file or choose a supported Money Source
-> Detect and confirm/create the Money Source plus sub-account(s) as evidence arrives
-> Optionally authorize the [CanCan iCloud Drive root](./0017-evidence-documents-source-ux.md#user-authorized-cancan-icloud-drive-root) or connect Gmail using Desktop OAuth + PKCE loopback
-> Optional statement password setup when a supported provider needs it
-> Review enabled capabilities and privacy-sensitive switches
-> Land in Command Center
```

MVP should not ask the user to choose a base currency. CanCan should render native values and source-provided valuations in the Money Overview.

## Local-first and encryption story

Reserve dedicated onboarding screens for:

```text
where the vault and evidence live
what stays on the device
what may leave the device when Gmail, AI, or a future connector is enabled
how vault locking and encryption protect stored data
what recovery can and cannot do
```

Use a simple animated data-flow/vault visual rather than legal copy or a technical architecture diagram. The user can expand `How it works` for detail.

Copy must be derived from accepted security/key/consent specs. Until those decisions are accepted, the screen structure is stable but exact encryption, key recovery, and cloud-data claims remain implementation-blocked.

## Setup review

Before entering Command Center, show one polished review screen with the current state of applicable capabilities:

```text
Local vault and lock
AI provider / cloud processing
Gmail connection
Automatic Gmail scan
CanCan iCloud Drive root (`Inbox` / `Backups`)
Automatically add qualified records
Optional FX-rate source
Update checks
Backup target / last successful backup
Crash reporting or diagnostics when later defined
```

Each row shows `On`, `Off`, `Not configured`, or `Needs attention`, a one-line consequence, and an edit action. Do not hide privacy-sensitive defaults or force the user to revisit earlier steps to understand what is enabled.

Saving the recovery file is recommended but may be deferred. Deferring it does not block entry into Command Center. Setup review and Settings keep recovery visibly `Not configured`; the Command Center `To do` banner list also keeps a direct `Save recovery file` action until saving succeeds. Cancellation or a failed save leaves the task visible and the Vault unconfigured.

## Network and telemetry boundary

MVP has no CanCan-owned application backend. Network access is capability-scoped and performed directly by the local client only after the user enables or configures that capability.

User-visible network capabilities may include:

```text
Gmail API through user OAuth
BYO AI provider through the user's configured endpoint/model, API key, and consent
optional product-defined read-only FX-rate source
optional update checks backed by GitHub Releases
future product-supported read-only connectors
```

Each capability appears separately in onboarding/setup review and Settings, explains what data leaves the device, and can be disabled without disabling the local vault.

MVP contains no product analytics or behavioral telemetry. If analytics is ever added, it must be explicit opt-in, default off, independently disableable, and absent from the vault's core functionality. Crash reporting follows the same explicit-consent rule when later defined.

## AI provider setup

MVP uses bring-your-own AI. The user configures a supported provider endpoint/model and API key; the local client calls that provider directly after consent.

A future release may add an optional CanCan-hosted AI relay with paid entitlement. That service is deliberately deferred: it is not required for local Vault use, must fit behind the same provider-adapter/capability boundary as BYO AI, and must not turn the current direct-provider path into a hosted dependency. Do not add relay endpoints, account/payment flows, entitlement storage, or service-specific runtime abstractions during MVP. Its exact identity, payment, minimal-state, retention, and privacy contracts require a separate design and release decision.

Rules:

- app should be usable enough to create a vault, create sources, and browse source documents without custom AI;
- parser-quality features should clearly explain when AI setup is needed;
- user-owned provider settings are the current supported path for privacy, cost transparency, and control;
- Vercel AI SDK may be used behind CanCan-owned adapters.

Normal onboarding exposes one simple `AI document analysis` setup. Its recommended mode uses a multimodal AI normalizer and lets the app combine the original document, native text extraction, and conditional local OCR without asking the user to understand those stages.

Advanced settings may enable separate extraction:

```text
Text recognition: local or configured OCR service
AI structuring: text-only or multimodal normalizer
```

The app chooses native text, OCR, both, or original page evidence using the accepted parser input-planning rules. The normal onboarding flow does not ask users to choose an OCR provider or a normalizer architecture. Exact cloud-data disclosure and consent text remain blocked by the Gmail/cloud-AI consent decision.

The future dedicated cloud-image OCR capability is separately disclosed and explicitly opt-in. Its manually imported cloud OCR payload/config and transport contract is owned by [`0004-parser-contract.md`](./0004-parser-contract.md). The reusable executor may perform the accepted request through an injected fetch boundary, but none of the user flow exists yet: Keychain or environment loading, Settings, Tauri/sidecar/UI wiring, disclosure UI, and the opt-in local smoke test remain unimplemented. It does not authorize a live provider call during development or Gmail attachment or message-body uploads.

## Gmail setup decision

Gmail MVP uses official Gmail API with Desktop OAuth Authorization Code Flow + PKCE + loopback redirect.

Computer-use/browser automation is not the primary Gmail architecture. It may be reconsidered later for non-Gmail bank portals or as an experimental fallback.

Gmail setup is not the first or required ingestion step. Onboarding should lead with `Add a file`, then offer the [CanCan iCloud Drive root](./0017-evidence-documents-source-ux.md#user-authorized-cancan-icloud-drive-root) and Gmail as optional convenience channels. Transaction-notification body capture is a separate opt-in from statement-attachment collection because it reads and retains different email content.

## Protected statement passwords

Some bank or card PDF statements require a password before text extraction or OCR.

MVP UX:

```text
If a downloaded/imported PDF is password protected:
  show a clear unlock prompt
  let user apply the password once
  offer optional secure save for that Money Source
  retry extraction after unlock
```

Security rules:

- statement passwords are optional;
- saved passwords go to OS secret storage / Keychain / Stronghold, never plain SQLite;
- SQLite may store only one secret reference id and status per Money Source;
- Gmail and manual imports for that Money Source reuse the saved password;
- a mismatch offers use once or update saved password; MVP stores no password history or unlocked duplicate PDF;
- passwords must not be logged, sent to AI, or included in backups by default;
- user can remove saved passwords from Settings.

## Animation and interaction

First-run should feel premium and technical without becoming ornamental.

Allowed design direction:

- smooth step transitions;
- animated local-device/vault/data-flow explanation;
- vault creation progress animation;
- source cards that feel interactive and tactile;
- subtle security/local-first visual cues;
- clear success states after each setup step;
- continuity animations when a configured source appears in the final Money Overview preview;
- reduced-motion fallback.

Avoid:

- slow blocking animations;
- decorative animation with no state meaning;
- complex onboarding that delays first useful action.

## Startup sequence after vault exists

```text
Open app
-> unlock vault
-> run schema compatibility check
-> apply allowed migrations or require upgrade path
-> load settings and supported provider registry
-> load configured secret references without exposing secret values to UI
-> find unfinished jobs
-> mark expired running jobs as queued
-> build resume plan
-> rescan the enabled CanCan root's `Inbox` child after unlock
-> optionally run enabled Gmail scans if auto-scan is on
-> land on Command Center with status modules
```

## Acceptance criteria

- First launch explains CanCan clearly.
- Onboarding has dedicated local-first and encryption/privacy explanation screens whose claims come from accepted security specs.
- The final setup review clearly shows which applicable capabilities and privacy-sensitive switches are on, off, or unconfigured.
- MVP network access is capability-scoped, user-authorized, direct from the local client, and not dependent on a CanCan backend.
- MVP has no analytics/behavioral telemetry; any future analytics or crash reporting is explicit opt-in and default off.
- Motion follows design tokens, communicates setup state, and has a reduced-motion fallback.
- User cannot accidentally create arbitrary unsupported providers.
- User can create a vault and reach first evidence capture without source/account preselection; a supported Money Source is confirmed or created before first commit.
- User may defer saving the recovery file, with an accurate persistent `Not configured` state and a later save action.
- MVP does not ask for base currency.
- AI setup is prominent and its current state is visible; whether setup may be skipped follows the unresolved first-run optionality decision.
- Gmail setup uses local-first Desktop OAuth + PKCE loopback flow.
- The first useful import does not require Gmail or a source/account preselection; optional CanCan-root setup links to the canonical `Inbox`/`Backups` contract and explains that the source folder remains outside the encrypted Vault.
- Transaction-notification email body capture is separately disclosed and enabled from attachment collection.
- Protected PDF statements can prompt for a password and optionally save it securely.
- Startup handles locked vault, migration checks, enabled root-`Inbox` rescan, optional Gmail scan, configured secrets, and unfinished jobs.
