use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand::{RngCore, rngs::OsRng};
use rusqlite::Connection;
use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    time::Instant,
};
use zeroize::Zeroizing;

const ENVELOPE_MAGIC: &[u8; 9] = b"CCSPIKE01";
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;

type SpikeResult<T> = Result<T, Box<dyn Error>>;

fn main() -> SpikeResult<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("storage-smoke") => {
            let root = required_path(args.next(), "storage-smoke <directory>")?;
            println!("{}", storage_smoke(&root)?);
        }
        Some("security-smoke") => {
            let root = required_path(args.next(), "security-smoke <directory>")?;
            println!("{}", security_smoke(&root)?);
        }
        Some("keychain-smoke") => println!("{}", keychain_smoke()?),
        Some(command) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown spike command: {command}"),
            )
            .into());
        }
        None => {
            tauri::Builder::default()
                .run(tauri::generate_context!())
                .map_err(|error| io::Error::other(format!("Tauri runtime failed: {error}")))?;
        }
    }
    Ok(())
}

fn required_path(value: Option<String>, usage: &str) -> SpikeResult<PathBuf> {
    value.map(PathBuf::from).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("usage: {usage}")).into()
    })
}

fn storage_smoke(root: &Path) -> SpikeResult<String> {
    fs::create_dir_all(root)?;
    let database_path = root.join("sqlcipher-fts5.sqlite");
    if database_path.exists() {
        fs::remove_file(&database_path)?;
    }

    let connection = Connection::open(&database_path)?;
    connection.pragma_update(None, "key", "cancan-feasibility-only")?;
    let cipher_version: String =
        connection.query_row("PRAGMA cipher_version", [], |row| row.get(0))?;
    if cipher_version.trim().is_empty() {
        return Err(io::Error::other("SQLCipher did not report a cipher version").into());
    }

    connection.execute_batch(
        "CREATE TABLE evidence (id INTEGER PRIMARY KEY, body TEXT NOT NULL);
         CREATE VIRTUAL TABLE evidence_search USING fts5(body);
         INSERT INTO evidence(body) VALUES ('cancan encrypted sentinel');
         INSERT INTO evidence_search(body) VALUES ('future finance evidence');",
    )?;
    let matches: i64 = connection.query_row(
        "SELECT count(*) FROM evidence_search WHERE evidence_search MATCH 'future'",
        [],
        |row| row.get(0),
    )?;
    if matches != 1 {
        return Err(io::Error::other("FTS5 query did not return the inserted row").into());
    }
    drop(connection);

    let database_bytes = fs::read(&database_path)?;
    if database_bytes
        .windows(b"cancan encrypted sentinel".len())
        .any(|window| window == b"cancan encrypted sentinel")
    {
        return Err(io::Error::other("plaintext sentinel was visible in SQLCipher file").into());
    }

    let wrong_key = Connection::open(&database_path)?;
    wrong_key.pragma_update(None, "key", "wrong-key")?;
    if wrong_key
        .query_row("SELECT count(*) FROM evidence", [], |row| {
            row.get::<_, i64>(0)
        })
        .is_ok()
    {
        return Err(io::Error::other("SQLCipher accepted the wrong key").into());
    }
    drop(wrong_key);

    let reopened = Connection::open(&database_path)?;
    reopened.pragma_update(None, "key", "cancan-feasibility-only")?;
    let rows: i64 = reopened.query_row("SELECT count(*) FROM evidence", [], |row| row.get(0))?;
    if rows != 1 {
        return Err(io::Error::other("SQLCipher reopen did not preserve the row").into());
    }
    drop(reopened);
    fs::remove_file(database_path)?;

    Ok(format!(
        "storage smoke passed: SQLCipher {cipher_version}; FTS5 match count {matches}"
    ))
}

fn security_smoke(root: &Path) -> SpikeResult<String> {
    fs::create_dir_all(root)?;
    let mut master_key = Zeroizing::new([0_u8; KEY_LEN]);
    OsRng.fill_bytes(master_key.as_mut());

    let mut salt = [0_u8; 16];
    OsRng.fill_bytes(&mut salt);
    let started = Instant::now();
    let password_key = derive_spike_key(b"feasibility-password", &salt)?;
    let argon2_ms = started.elapsed().as_millis();
    let password_wrapper = seal(password_key.as_ref(), master_key.as_ref())?;
    let recovered_from_password = open(password_key.as_ref(), &password_wrapper)?;
    if recovered_from_password.as_slice() != master_key.as_ref() {
        return Err(io::Error::other("password wrapper did not recover the master key").into());
    }

    let wrong_password_key = derive_spike_key(b"wrong-password", &salt)?;
    if open(wrong_password_key.as_ref(), &password_wrapper).is_ok() {
        return Err(io::Error::other("password wrapper accepted a wrong password").into());
    }

    let mut recovery_key = Zeroizing::new([0_u8; KEY_LEN]);
    OsRng.fill_bytes(recovery_key.as_mut());
    let recovery_wrapper = seal(recovery_key.as_ref(), master_key.as_ref())?;
    let recovered_from_file = open(recovery_key.as_ref(), &recovery_wrapper)?;
    if recovered_from_file.as_slice() != master_key.as_ref() {
        return Err(io::Error::other("recovery wrapper did not recover the master key").into());
    }

    let evidence_plaintext = b"synthetic statement evidence";
    let encrypted_evidence = seal(master_key.as_ref(), evidence_plaintext)?;
    let encrypted_path = root.join("synthetic-statement.enc");
    fs::write(&encrypted_path, &encrypted_evidence)?;
    let on_disk = fs::read(&encrypted_path)?;
    if on_disk
        .windows(evidence_plaintext.len())
        .any(|window| window == evidence_plaintext)
    {
        return Err(io::Error::other("plaintext evidence was visible in encrypted file").into());
    }
    if open(master_key.as_ref(), &on_disk)?.as_slice() != evidence_plaintext {
        return Err(io::Error::other("encrypted evidence did not round trip").into());
    }

    let mut tampered = on_disk;
    let last = tampered
        .last_mut()
        .ok_or_else(|| io::Error::other("encrypted envelope was empty"))?;
    *last ^= 1;
    if open(master_key.as_ref(), &tampered).is_ok() {
        return Err(io::Error::other("tampered evidence authenticated successfully").into());
    }
    fs::remove_file(encrypted_path)?;

    Ok(format!(
        "security smoke passed: Argon2id derivation {argon2_ms} ms; password, recovery, file encryption, and tamper rejection passed"
    ))
}

fn derive_spike_key(password: &[u8], salt: &[u8]) -> SpikeResult<Zeroizing<[u8; KEY_LEN]>> {
    let params = Params::new(19_456, 2, 1, Some(KEY_LEN))
        .map_err(|error| io::Error::other(format!("Argon2 params failed: {error}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    argon2
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|error| io::Error::other(format!("Argon2 derivation failed: {error}")))?;
    Ok(key)
}

fn seal(key: &[u8], plaintext: &[u8]) -> SpikeResult<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::other("invalid encryption key length"))?;
    let mut nonce = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|_| io::Error::other("encryption failed"))?;

    let mut envelope = Vec::with_capacity(ENVELOPE_MAGIC.len() + NONCE_LEN + ciphertext.len());
    envelope.extend_from_slice(ENVELOPE_MAGIC);
    envelope.extend_from_slice(&nonce);
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

fn open(key: &[u8], envelope: &[u8]) -> SpikeResult<Vec<u8>> {
    let header_len = ENVELOPE_MAGIC.len() + NONCE_LEN;
    if envelope.len() <= header_len || &envelope[..ENVELOPE_MAGIC.len()] != ENVELOPE_MAGIC {
        return Err(io::Error::other("invalid encrypted envelope").into());
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::other("invalid decryption key length"))?;
    let nonce = XNonce::from_slice(&envelope[ENVELOPE_MAGIC.len()..header_len]);
    cipher
        .decrypt(nonce, &envelope[header_len..])
        .map_err(|_| io::Error::other("decryption or authentication failed").into())
}

fn keychain_smoke() -> SpikeResult<String> {
    let username = format!("spike-{}", std::process::id());
    let entry = keyring::Entry::new("app.cancan.feasibility", &username)?;
    let mut secret = Zeroizing::new([0_u8; KEY_LEN]);
    OsRng.fill_bytes(secret.as_mut());
    entry.set_secret(secret.as_ref())?;
    let loaded = entry.get_secret();
    let cleanup = entry.delete_credential();
    let loaded = loaded?;
    cleanup?;
    if loaded.as_slice() != secret.as_ref() {
        return Err(io::Error::other("Keychain returned different bytes").into());
    }
    Ok("keychain smoke passed: write/read/delete round trip".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlcipher_and_fts5_work_together() {
        let root = tempfile::tempdir().expect("temp directory");
        storage_smoke(root.path()).expect("storage smoke");
    }

    #[test]
    fn encryption_password_and_recovery_wrappers_reject_tampering() {
        let root = tempfile::tempdir().expect("temp directory");
        security_smoke(root.path()).expect("security smoke");
    }
}
