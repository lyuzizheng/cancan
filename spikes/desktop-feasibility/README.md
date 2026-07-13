# Desktop Feasibility Spike

This directory is disposable evidence for the `desktop-feasibility` implementation slice. It is not production app code.

It tests only the declared slice gates:

- a minimal Tauri 2 desktop build;
- bundled SQLCipher plus FTS5 in one SQLite build;
- authenticated file encryption and tamper rejection;
- password and recovery wrappers around one random vault key;
- a write/read/delete round trip through the macOS Keychain.

The cryptographic format and Argon2id parameters in this spike are deliberately not a production compatibility contract.

Run the complete gate from this directory after installing the Tauri prerequisites:

```text
pnpm spike:verify
```

Each smoke command uses disposable data under `.tmp/`. The Keychain command deletes its uniquely named test entry before returning.

See [EVIDENCE.md](EVIDENCE.md) for the environment, observed results, accepted conclusion, and decisions this spike does not make.
