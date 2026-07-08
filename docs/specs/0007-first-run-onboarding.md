# 0007. First-Run and Onboarding Spec

## Goal

Define the first-launch experience and startup sequence for CanCan.

The first-run experience must communicate technology, safety, and local ownership while staying practical.

## Opening message

The opening screen should feel modern, warm, and secure.

Draft positioning copy:

```text
CanCan is your personal local money can.
Bring your own AI, connect trusted money sources, and turn statements, emails, and exports into a private financial vault you control.
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
-> Create local vault
-> Choose vault password / local key setup
-> AI provider setup
-> Create first money source from supported providers
-> Create source sub-account(s)
-> Configure Gmail using Desktop OAuth + PKCE loopback, or import manually
-> Optional statement password setup when a supported provider needs it
-> Land in Command Center
```

MVP should not ask the user to choose a base currency. CanCan should render native values and source-provided valuations in the Money Overview.

## AI provider setup

CanCan may provide a default AI provider path, but should strongly prompt the user to bring their own key/provider.

Rules:

- app should be usable enough to create vault, sources, and library without custom AI;
- parser-quality features should clearly explain when AI setup is needed;
- user-owned provider settings should be preferred for privacy, cost transparency, and control;
- Vercel AI SDK may be used behind CanCan-owned adapters.

## Gmail setup decision

Gmail MVP uses official Gmail API with Desktop OAuth Authorization Code Flow + PKCE + loopback redirect.

Computer-use/browser automation is not the primary Gmail architecture. It may be reconsidered later for non-Gmail bank portals or as an experimental fallback.

## Protected statement passwords

Some bank or card PDF statements require a password before text extraction or OCR.

MVP UX:

```text
If a downloaded/imported PDF is password protected:
  show a clear unlock prompt
  let user apply the password once
  offer optional secure save for that provider/account pattern
  retry extraction after unlock
```

Security rules:

- statement passwords are optional;
- saved passwords go to OS secret storage / Keychain / Stronghold, never plain SQLite;
- SQLite may store only a secret reference id and provider/account scope;
- passwords must not be logged, sent to AI, or included in backups by default;
- user can remove saved passwords from Settings.

## Animation and interaction

First-run should feel premium and technical without becoming ornamental.

Allowed design direction:

- smooth step transitions;
- vault creation progress animation;
- source cards that feel interactive and tactile;
- subtle security/local-first visual cues;
- clear success states after each setup step;
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
-> optionally run enabled Gmail scans if auto-scan is on
-> land on Command Center with status modules
```

## Acceptance criteria

- First launch explains CanCan clearly.
- User cannot accidentally create arbitrary unsupported providers.
- User can create a vault and at least one supported money source.
- MVP does not ask for base currency.
- AI setup is prominent but not a hard blocker for all app use.
- Gmail setup uses local-first Desktop OAuth + PKCE loopback flow.
- Protected PDF statements can prompt for a password and optionally save it securely.
- Startup handles locked vault, migration checks, optional Gmail scan, configured secrets, and unfinished jobs.
