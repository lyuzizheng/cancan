# ADR 0001: Use Tauri + React + SQLite for the local-first app

## Status

Proposed

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
```

Do not use Rails/Sure as the app base. Use Sure only as a domain/product reference.

Do not use SwiftUI for MVP.

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
