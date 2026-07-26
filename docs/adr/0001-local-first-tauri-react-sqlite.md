# ADR 0001: Use Tauri + React + SQLite for the local-first app

## Status

Accepted

## Context

CanCan is a personal finance reconciliation vault. It should be local-first, desktop-first, and store data in a local encrypted SQLite vault with file attachments and encrypted backup snapshots.

The developer is comfortable with React, backend frameworks, and Flutter, but not Swift/SwiftUI. The app will rely heavily on coding AI tools, so the main business code should be easy to generate, review, and refactor.

## Decision

Use:

```text
Tauri app shell
React + TypeScript UI
TypeScript core domain engine
SQLite/SQLCipher local database
Tauri/Rust privileged commands for filesystem, database, backup, and secrets
Tauri-controlled bundled Node worker mode for executing the pure TypeScript core behind the host boundary
```

The deterministic core worker mode has no AI/model, filesystem, database, network, secret, or renderer capability. Rust supplies bounded typed inputs, validates the protocol result, and remains the only owner of SQLCipher transactions and durable financial writes. This avoids a second implementation of canonical validation, reconciliation, and ledger-construction rules in Rust.

Do not use Rails/Sure as the app base. Use Sure only as a domain/product reference.

Do not use SwiftUI for MVP.

## Feasibility evidence

The disposable [desktop feasibility spike](../../spikes/desktop-feasibility/EVIDENCE.md) passed on macOS arm64 on 2026-07-13:

```text
Tauri 2 debug desktop build
bundled SQLCipher 4.14.0 with FTS5 enabled
encrypted database reopen and wrong-key rejection
authenticated file encryption and tamper rejection
password and recovery wrappers around one master key
macOS Keychain binary-secret write/read/delete
```

This spike accepted the runtime and package-boundary architecture but did not itself prove a production cryptographic format, exact Argon2id parameters, plaintext-viewing policy, secret-store behavior, or release/signing configuration. The later [Vault security validation](../../spikes/vault-security-validation/EVIDENCE.md) accepted the versioned Argon2id profiles, authenticated envelope fixtures, macOS Keychain scope, no-temporary-plaintext viewer, deletion recovery, and restore locator switch. Release signing and real macOS `x86_64` qualification remain later release gates.

## Consequences

Positive:

```text
local-first architecture
React developer productivity
TypeScript shared models
small desktop app shell
explicit permission boundary
SQLite-first storage
future mobile options remain open
```

Negative:

```text
cannot directly reuse Sure Rails code
some Tauri/Rust learning required
mobile app will likely need separate shell/adapters
SQLCipher integration needs care
```

## Alternatives considered

### Rails/Sure fork

Good for a self-hosted web app, but not aligned with local-first SQLite/iCloud/no-server goals.

### SwiftUI

Best Apple-native integration, but high learning cost and less aligned with the team's current skills.

### Electron

Fastest JS/Node development path, but heavier and requires stricter security discipline.

### Flutter

Strong cross-platform UI, but React/TypeScript gives better reuse with current skillset and AI coding workflow.
