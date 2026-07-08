# Security And Data Rules

- Vault data is local-first and encrypted at rest from v1.
- Store secrets outside plain SQLite.
- Gmail uses official read-only OAuth/API for MVP.
- Saved statement PDF passwords are optional and live only in OS secret storage; SQLite stores references.
- Cloud AI upload is opt-in and must not include secrets.
- Backups do not include OAuth/API secrets, AI keys, vault key material, or statement PDF passwords by default.
- Ask the user before changing irreversible data shape, AI authority, secret handling, backup compatibility, or provider support claims.
