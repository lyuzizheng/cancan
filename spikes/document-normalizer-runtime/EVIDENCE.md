# Document normalizer runtime evidence

Verified on 2026-07-14 with Node 24.18.0 on macOS arm64
(`aarch64-apple-darwin`). The dependency lock pins Vercel AI SDK 6.0.224,
Pi Agent Core 0.80.6, and Pi AI 0.80.6.

## Runtime comparison

All three paths accepted the same six-record synthetic proposal with SHA-256
`8aabc71120c4fc73d56a6be8d36d4035a2f259202b1d98d2793ab48a97a9d036`.

| Runtime | Model steps | Tool calls | Mock tokens | Result |
| --- | ---: | ---: | ---: | --- |
| Single pass | 1 | 0 | 15 | accepted |
| ToolLoopAgent | 5 | 4 | 75 | accepted |
| Pi Agent Core | 4 | 4 | 60 | accepted |

Mock tokens are the fixed 15-token usage attached to each deterministic model
step; they are comparison accounting, not a live-provider cost estimate. Local
elapsed time is recorded in generated `dist/evidence.json`, but is not a model
latency benchmark.

The eight deterministic tests also prove:

- exactly seven strict, job-scoped tools derived into both framework adapters, with bounded inputs and runtime-validated/capped outputs;
- free text and nonexistent/forbidden capability rejection without secret echo;
- production proposal validation on every submission and a two-submission limit;
- native/OCR conflict and ungrounded-value rejection;
- deterministic eight-step exhaustion; and
- cancellation in both agent frameworks.

## Tauri and Node boundary

The build bundles the selected worker and AI SDK dependency graph into a 2,813,069
byte CommonJS artifact, injects it into the pinned Node binary as a single
executable, ad-hoc signs it, and bundles it as a Tauri external binary. The debug
arm64 measurements were:

| Evidence | Result |
| --- | --- |
| Development worker startup | about 126-140 ms in repeated gates |
| Packaged worker startup | about 131-149 ms median; observed runs 129-184 ms warm with cold verification outliers of 1.7-2.2 s |
| SEA sidecar size | 122,852,864 bytes |
| Debug `.app` bundle size | 158,996,559 bytes |
| Clean shutdown | passed in all runs |
| Cancellation | passed in development and packaged workers |
| Crash isolation | child exited 70; host continued |
| Tauri host smoke | launch, ping/pong, and shutdown passed |
| Protocol secret rejection | extra secret field rejected without echo |
| Environment secret handling | direct-worker key value was not echoed; Tauri cleared its inherited environment before sidecar launch |
| Production notarization | not tested; requires the release signing identity |
| Spike dependency license inventory | generated; over-includes comparison dependencies not present in the selected sidecar; 49,543 bytes, SHA-256 `d695abdaef685efb7f943a371ebe16360d68a83de59cf9304c3bea9a00983e4e` |

The sidecar is a trusted packaged process boundary, not a permission sandbox. The
Tauri host remains responsible for user-selected file access and OS-secret
retrieval. Exact live-provider credential delivery, redaction, release signing,
and notarization remain owned by their later security/provider/release slices.
The release slice must generate an artifact-specific SBOM/license inventory rather
than treating this conservative workspace inventory as the shipped artifact list.

## Selection

Select single-pass structured normalization for the initial production runtime,
executed in a Tauri-controlled bundled Node sidecar. The deterministic evidence
found no accuracy or recovery advantage for either agent loop, while each required
four or five model steps instead of one and added a framework lifecycle and
dependency surface.

ToolLoopAgent and Pi Agent Core remain unselected comparison options. Reconsider
an agent loop only if the real qualification fixture suite demonstrates a material
accuracy or bounded-repair advantage over the same single-pass contract.

This selection establishes the runtime and packaging boundary only. It does not
qualify any parser for auto-commit and does not authorize production document,
secret, or ledger behavior.
