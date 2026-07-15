# Document normalizer runtime spike

This disposable spike compares the Vercel AI SDK `ToolLoopAgent`, Pi Agent Core,
and a single-pass structured-normalization baseline against the same synthetic
fixture, proposal schema, production validator, and deterministic mock transcript.
It also packages the selected boundary as a Node 24 single executable sidecar and
proves that a Tauri host can control it without a user-installed runtime.

It does not call a live model, implement a production provider adapter, qualify a
parser profile, or prove release signing/notarization.

Run the complete reproducible gate on the supported macOS arm64 development target:

```sh
pnpm --dir spikes/document-normalizer-runtime spike:verify
```

Generated binaries, build output, license inventory, and machine-readable evidence
stay ignored under `dist/`, `src-tauri/binaries/`, and `src-tauri/target/`.
The reviewed evidence and selection are recorded in [EVIDENCE.md](./EVIDENCE.md).
