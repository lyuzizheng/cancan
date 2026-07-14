# ADR 0003: Use single-pass normalization in a bundled Node sidecar

## Status

Accepted

## Context

CanCan needs a mandatory AI normalizer that emits the parser proposal contract
without gaining ledger authority. The initial candidates were a single structured
generation, Vercel AI SDK `ToolLoopAgent`, and Pi Agent Core. The TypeScript AI
runtime also needs a reproducible Tauri packaging boundary that does not require a
user-installed runtime or expose privileged capabilities to the renderer.

## Decision

Use single-pass structured normalization for the initial production implementation.
Run it in a trusted Node worker/sidecar bundled and controlled by Tauri. On the
supported macOS arm64 development target, package the pinned Node 24 runtime as a
single executable external binary.

The Tauri/Rust host owns user-selected file access and OS-secret retrieval. The
worker is launched with a cleared inherited environment and limited to the current parse job, runtime configuration, the shared
proposal schema, and deterministic validators. It has no shell, generic filesystem,
arbitrary network tool, database, secret tool, or ledger capability. The sidecar is
process and crash separation, not a permission sandbox.

Do not adopt ToolLoopAgent or Pi Agent Core until the real qualification suite
demonstrates a material accuracy or bounded-repair advantage over the same
single-pass proposal and validation contract.

## Evidence

The disposable [document normalizer runtime spike](../../spikes/document-normalizer-runtime/EVIDENCE.md)
passed on macOS arm64 on 2026-07-14. All three runtime paths emitted the same
accepted six-record proposal. Single-pass used one deterministic model step;
ToolLoopAgent used five and Pi Agent Core used four.

The spike also proved frozen dependency resolution, Node single-executable
bundling, ad-hoc signing, Tauri external-binary launch, protocol framing, inherited-
environment clearing, secret-field rejection without echo, cancellation, clean
shutdown, crash isolation, debug bundle size, and a conservative comparison-
workspace license inventory.

This accepts the runtime and packaging architecture. It does not qualify a live
model or parser profile, define production credential transport/redaction, produce
an artifact-specific SBOM/license inventory, or accept Developer ID signing and
notarization; those remain gates in their owning provider, security, and release
slices.

## Consequences

Positive:

```text
one initial model call instead of a multi-step agent loop
one proposal and validation contract across current and future runtimes
no renderer-side Node or privileged Tauri capability exposure
no user-installed Node or sandbox runtime
isolated cancellation and crash lifecycle controlled by Tauri
```

Negative:

```text
the bundled Node runtime materially increases application size
Node single-executable packaging and Tauri sidecar integration need release care
single-pass cannot use iterative tool recovery unless later evidence justifies it
production signing, notarization, and credential delivery remain separate gates
```
