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

Users can create accounts/sub-accounts under supported money sources. Custom/manual sources can exist later, but should not be confused with supported automated integrations.

## First-run flow

Recommended flow:

```text
Welcome / product promise
-> Create local vault
-> Choose vault password / local key setup
-> Choose base currency, default SGD
-> AI provider setup
-> Create first money source from supported providers
-> Create source sub-account(s)
-> Configure Gmail using Desktop OAuth + PKCE loopback, or import manually
-> Land in Command Center
```

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
- AI setup is prominent but not a hard blocker for all app use.
- Gmail setup uses local-first Desktop OAuth + PKCE loopback flow.
- Startup handles locked vault, migration checks, optional Gmail scan, and unfinished jobs.
