# Vault security validation

Disposable evidence for the `vault-security-validation` implementation slice.
It does not provide production Vault APIs or persist real user data.

The macOS CI gate validates:

- both accepted Argon2id profiles and the 750 ms profile-selection rule;
- a versioned XChaCha20-Poly1305 envelope with authenticated metadata;
- deterministic architecture-portable envelope bytes and tamper rejection;
- one replaceable statement-PDF password per Money Source in Keychain;
- encrypted PDF decryption and Core Graphics rendering entirely in memory;
- idempotent source-file tombstone recovery at each destructive crash boundary;
- restore into a validated inactive Vault followed by an atomic locator switch.

The complete gate is owned by `.github/workflows/vault-security-validation.yml`.
Run `scripts/verify.sh` locally only when diagnosing or changing this spike.
