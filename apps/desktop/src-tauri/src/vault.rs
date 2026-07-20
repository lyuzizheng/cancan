use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"CCENV001";
const VERSION: u8 = 1;
const PURPOSE_FILE: u8 = 1;
const PURPOSE_PASSWORD_WRAPPER: u8 = 2;
const ALGORITHM_XCHACHA20_POLY1305: u8 = 1;
const KDF_NONE: u8 = 0;
const KDF_RFC9106_LOW_MEMORY_V1: u8 = 1;
const KDF_OWASP_MINIMUM_V1: u8 = 2;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;
const HEADER_FIXED_LEN: usize = 24;
const FILE_KEY_CONTEXT: &[u8] = b"cancan:file:v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum KdfProfile {
    Rfc9106LowMemoryV1 = KDF_RFC9106_LOW_MEMORY_V1,
    OwaspMinimumV1 = KDF_OWASP_MINIMUM_V1,
}

impl KdfProfile {
    fn params(self) -> io::Result<Params> {
        let (memory, iterations, lanes) = match self {
            Self::Rfc9106LowMemoryV1 => (65_536, 3, 4),
            Self::OwaspMinimumV1 => (19_456, 2, 1),
        };
        Params::new(memory, iterations, lanes, Some(KEY_LEN))
            .map_err(|error| io::Error::other(format!("invalid Argon2 profile: {error}")))
    }
}

#[derive(Debug)]
struct ParsedEnvelope<'a> {
    purpose: u8,
    profile: u8,
    salt: &'a [u8],
    nonce: &'a [u8],
    ciphertext: &'a [u8],
    authenticated_header: &'a [u8],
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StoredFile {
    pub(crate) byte_size: u64,
    pub(crate) created: bool,
    pub(crate) encrypted_locator: String,
    pub(crate) file_sha256: String,
}

pub(crate) struct PreparedSource {
    byte_size: u64,
    file_sha256: String,
    plaintext: Zeroizing<Vec<u8>>,
}

impl PreparedSource {
    pub(crate) fn file_sha256(&self) -> &str {
        &self.file_sha256
    }
}

#[derive(Debug)]
pub(crate) struct FileVault {
    root: PathBuf,
}

impl FileVault {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[cfg(test)]
    pub(crate) fn store(
        &self,
        master_key: &[u8; KEY_LEN],
        source_path: &Path,
    ) -> io::Result<StoredFile> {
        let source = Self::prepare(source_path)?;
        self.store_prepared(master_key, &source, false)
    }

    pub(crate) fn prepare(source_path: &Path) -> io::Result<PreparedSource> {
        let plaintext = Zeroizing::new(fs::read(source_path)?);
        let byte_size = u64::try_from(plaintext.len())
            .map_err(|_| io::Error::other("source file size does not fit u64"))?;
        let file_sha256 = hex_digest(&plaintext);
        Ok(PreparedSource {
            byte_size,
            file_sha256,
            plaintext,
        })
    }

    pub(crate) fn store_prepared(
        &self,
        master_key: &[u8; KEY_LEN],
        source: &PreparedSource,
        replace_existing: bool,
    ) -> io::Result<StoredFile> {
        let file_sha256 = &source.file_sha256;
        let encrypted_locator = format!("files/{file_sha256}.ccenv");
        let target = self.resolve_locator(&encrypted_locator)?;
        let directory = target
            .parent()
            .ok_or_else(|| io::Error::other("encrypted file has no parent directory"))?;
        fs::create_dir_all(directory)?;

        let file_key = derive_file_key(master_key)?;
        if replace_existing && target.exists() {
            fs::remove_file(&target)?;
            File::open(directory)?.sync_all()?;
        }
        let created = !target.exists();
        if target.exists() {
            let existing = fs::read(&target)?;
            let opened = open_file_envelope(&file_key, &existing)?;
            if hex_digest(&opened) != *file_sha256 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "existing encrypted file does not match its source hash",
                ));
            }
        } else {
            let envelope = seal_file_envelope(&file_key, &source.plaintext)?;
            self.write_atomically(directory, &target, &envelope)?;
        }

        Ok(StoredFile {
            byte_size: source.byte_size,
            created,
            encrypted_locator,
            file_sha256: file_sha256.clone(),
        })
    }

    pub(crate) fn open_in_memory(
        &self,
        master_key: &[u8; KEY_LEN],
        encrypted_locator: &str,
    ) -> io::Result<Zeroizing<Vec<u8>>> {
        let path = self.resolve_locator(encrypted_locator)?;
        let envelope = fs::read(path)?;
        let file_key = derive_file_key(master_key)?;
        open_file_envelope(&file_key, &envelope)
    }

    pub(crate) fn verifies(
        &self,
        master_key: &[u8; KEY_LEN],
        encrypted_locator: &str,
        expected_sha256: &str,
    ) -> io::Result<bool> {
        let path = self.resolve_locator(encrypted_locator)?;
        let envelope = match fs::read(path) {
            Ok(envelope) => envelope,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
        let file_key = derive_file_key(master_key)?;
        let plaintext = match open_file_envelope(&file_key, &envelope) {
            Ok(plaintext) => plaintext,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => return Ok(false),
            Err(error) => return Err(error),
        };
        Ok(hex_digest(&plaintext) == expected_sha256)
    }

    pub(crate) fn remove(&self, encrypted_locator: &str) -> io::Result<()> {
        let path = self.resolve_locator(encrypted_locator)?;
        match fs::remove_file(path) {
            Ok(()) => {
                let directory = self.root.join("files");
                File::open(directory)?.sync_all()?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub(crate) fn remove_unreferenced(
        &self,
        referenced_locators: &HashSet<String>,
    ) -> io::Result<()> {
        let directory = self.root.join("files");
        if !directory.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let locator = format!("files/{name}");
            let stale_temporary = name.starts_with(".import-") && name.ends_with(".tmp");
            let orphaned_envelope =
                name.ends_with(".ccenv") && !referenced_locators.contains(&locator);
            if stale_temporary || orphaned_envelope {
                fs::remove_file(entry.path())?;
            }
        }
        File::open(directory)?.sync_all()
    }

    fn resolve_locator(&self, locator: &str) -> io::Result<PathBuf> {
        let relative = Path::new(locator);
        let mut components = relative.components();
        if components.next() != Some(Component::Normal("files".as_ref()))
            || components.clone().count() != 1
            || components.any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid encrypted file locator",
            ));
        }
        Ok(self.root.join(relative))
    }

    fn write_atomically(&self, directory: &Path, target: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut random = [0_u8; 8];
        OsRng.fill_bytes(&mut random);
        let temporary = directory.join(format!(".import-{}.tmp", hex_digest(&random)));
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, target)?;
            File::open(directory)?.sync_all()
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn derive_file_key(master_key: &[u8; KEY_LEN]) -> io::Result<Zeroizing<[u8; KEY_LEN]>> {
    let hkdf = Hkdf::<Sha256>::new(None, master_key);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    hkdf.expand(FILE_KEY_CONTEXT, key.as_mut())
        .map_err(|_| io::Error::other("file key derivation failed"))?;
    Ok(key)
}

fn seal_file_envelope(key: &[u8; KEY_LEN], plaintext: &[u8]) -> io::Result<Vec<u8>> {
    let mut nonce = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    seal_envelope_with_nonce(PURPOSE_FILE, KDF_NONE, &[], key, &nonce, plaintext)
}

#[cfg(test)]
fn seal_file_envelope_with_nonce(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
) -> io::Result<Vec<u8>> {
    seal_envelope_with_nonce(PURPOSE_FILE, KDF_NONE, &[], key, nonce, plaintext)
}

pub(crate) fn create_password_wrapper(
    password: &[u8],
    master_key: &[u8; KEY_LEN],
) -> io::Result<Vec<u8>> {
    let mut salt = [0_u8; 16];
    OsRng.fill_bytes(&mut salt);
    let started = Instant::now();
    let primary_key = derive_password_key(password, &salt, KdfProfile::Rfc9106LowMemoryV1)?;
    let (profile, wrapping_key) =
        if select_kdf_profile(started.elapsed()) == KdfProfile::Rfc9106LowMemoryV1 {
            (KdfProfile::Rfc9106LowMemoryV1, primary_key)
        } else {
            (
                KdfProfile::OwaspMinimumV1,
                derive_password_key(password, &salt, KdfProfile::OwaspMinimumV1)?,
            )
        };
    let mut nonce = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    seal_envelope_with_nonce(
        PURPOSE_PASSWORD_WRAPPER,
        profile as u8,
        &salt,
        &wrapping_key,
        &nonce,
        master_key,
    )
}

fn select_kdf_profile(primary_elapsed: Duration) -> KdfProfile {
    if primary_elapsed <= Duration::from_millis(750) {
        KdfProfile::Rfc9106LowMemoryV1
    } else {
        KdfProfile::OwaspMinimumV1
    }
}

pub(crate) fn password_wrapper_profile(envelope: &[u8]) -> io::Result<KdfProfile> {
    let parsed = parse_envelope(envelope)?;
    if parsed.purpose != PURPOSE_PASSWORD_WRAPPER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid password wrapper purpose",
        ));
    }
    if parsed.ciphertext.len() != KEY_LEN + 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid password wrapper payload length",
        ));
    }
    match parsed.profile {
        KDF_RFC9106_LOW_MEMORY_V1 => Ok(KdfProfile::Rfc9106LowMemoryV1),
        KDF_OWASP_MINIMUM_V1 => Ok(KdfProfile::OwaspMinimumV1),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid password wrapper profile",
        )),
    }
}

pub(crate) fn open_password_wrapper(
    envelope: &[u8],
    password: &[u8],
) -> io::Result<Zeroizing<[u8; KEY_LEN]>> {
    let parsed = parse_envelope(envelope)?;
    let profile = password_wrapper_profile(envelope)?;
    let key = derive_password_key(password, parsed.salt, profile)?;
    let plaintext = open_envelope(envelope, PURPOSE_PASSWORD_WRAPPER, &key)?;
    let master_key: [u8; KEY_LEN] = plaintext.as_slice().try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "password wrapper contains an invalid master key",
        )
    })?;
    Ok(Zeroizing::new(master_key))
}

fn derive_password_key(
    password: &[u8],
    salt: &[u8],
    profile: KdfProfile,
) -> io::Result<Zeroizing<[u8; KEY_LEN]>> {
    if salt.len() != 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "password wrapper salt must be 16 bytes",
        ));
    }
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, profile.params()?);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    argon2
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|error| io::Error::other(format!("Argon2 derivation failed: {error}")))?;
    Ok(key)
}

#[cfg(test)]
fn seal_password_wrapper_with_nonce(
    password: &[u8],
    master_key: &[u8; KEY_LEN],
    profile: KdfProfile,
    salt: &[u8; 16],
    nonce: &[u8; NONCE_LEN],
) -> io::Result<Vec<u8>> {
    let key = derive_password_key(password, salt, profile)?;
    seal_envelope_with_nonce(
        PURPOSE_PASSWORD_WRAPPER,
        profile as u8,
        salt,
        &key,
        nonce,
        master_key,
    )
}

fn seal_envelope_with_nonce(
    purpose: u8,
    profile: u8,
    salt: &[u8],
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
) -> io::Result<Vec<u8>> {
    let valid_salt = (profile == KDF_NONE && salt.is_empty())
        || (matches!(profile, KDF_RFC9106_LOW_MEMORY_V1 | KDF_OWASP_MINIMUM_V1)
            && salt.len() == 16);
    if !valid_salt {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid envelope KDF salt",
        ));
    }
    let ciphertext_len = plaintext
        .len()
        .checked_add(16)
        .ok_or_else(|| io::Error::other("ciphertext length overflow"))?;
    let mut header = Vec::with_capacity(HEADER_FIXED_LEN + salt.len() + NONCE_LEN + ciphertext_len);
    header.extend_from_slice(MAGIC);
    header.push(VERSION);
    header.push(purpose);
    header.push(ALGORITHM_XCHACHA20_POLY1305);
    header.push(profile);
    header.extend_from_slice(&(salt.len() as u16).to_be_bytes());
    header.extend_from_slice(&(NONCE_LEN as u16).to_be_bytes());
    header.extend_from_slice(&(ciphertext_len as u64).to_be_bytes());
    header.extend_from_slice(salt);
    header.extend_from_slice(nonce);

    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::other("invalid envelope encryption key"))?;
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad: &header,
            },
        )
        .map_err(|_| io::Error::other("envelope encryption failed"))?;
    header.extend_from_slice(&ciphertext);
    Ok(header)
}

fn open_file_envelope(key: &[u8; KEY_LEN], envelope: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    let parsed = parse_envelope(envelope)?;
    if parsed.purpose != PURPOSE_FILE || parsed.profile != KDF_NONE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid file envelope header",
        ));
    }
    open_envelope(envelope, PURPOSE_FILE, key)
}

fn parse_envelope(envelope: &[u8]) -> io::Result<ParsedEnvelope<'_>> {
    if envelope.len() < HEADER_FIXED_LEN
        || &envelope[..8] != MAGIC
        || envelope[8] != VERSION
        || !matches!(envelope[9], 1..=4)
        || envelope[10] != ALGORITHM_XCHACHA20_POLY1305
        || !matches!(
            envelope[11],
            KDF_NONE | KDF_RFC9106_LOW_MEMORY_V1 | KDF_OWASP_MINIMUM_V1
        )
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid envelope header",
        ));
    }
    let salt_len = usize::from(u16::from_be_bytes([envelope[12], envelope[13]]));
    let nonce_len = usize::from(u16::from_be_bytes([envelope[14], envelope[15]]));
    let ciphertext_len = usize::try_from(u64::from_be_bytes(
        envelope[16..24]
            .try_into()
            .expect("fixed envelope header length"),
    ))
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "ciphertext length overflow"))?;
    if nonce_len != NONCE_LEN
        || (envelope[11] == KDF_NONE && salt_len != 0)
        || (envelope[11] != KDF_NONE && salt_len != 16)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid envelope lengths",
        ));
    }
    let header_len = HEADER_FIXED_LEN
        .checked_add(salt_len)
        .and_then(|length| length.checked_add(nonce_len))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "header length overflow"))?;
    let expected_len = header_len
        .checked_add(ciphertext_len)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "envelope length overflow"))?;
    if envelope.len() != expected_len || ciphertext_len < 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid envelope size",
        ));
    }
    Ok(ParsedEnvelope {
        purpose: envelope[9],
        profile: envelope[11],
        salt: &envelope[HEADER_FIXED_LEN..HEADER_FIXED_LEN + salt_len],
        nonce: &envelope[HEADER_FIXED_LEN + salt_len..header_len],
        ciphertext: &envelope[header_len..],
        authenticated_header: &envelope[..header_len],
    })
}

fn open_envelope(
    envelope: &[u8],
    expected_purpose: u8,
    key: &[u8; KEY_LEN],
) -> io::Result<Zeroizing<Vec<u8>>> {
    let parsed = parse_envelope(envelope)?;
    if parsed.purpose != expected_purpose {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "wrong envelope purpose",
        ));
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::other("invalid envelope decryption key"))?;
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(parsed.nonce),
            Payload {
                msg: parsed.ciphertext,
                aad: parsed.authenticated_header,
            },
        )
        .map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "envelope authentication failed")
        })?;
    Ok(Zeroizing::new(plaintext))
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_ccenv001_and_opens_only_in_memory() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source = root.path().join("statement.pdf");
        let plaintext = b"%PDF-1.7 synthetic statement evidence";
        fs::write(&source, plaintext).expect("write source fixture");
        let vault = FileVault::new(root.path().join("vault"));
        let key = [0x41; KEY_LEN];

        let stored = vault.store(&key, &source).expect("store encrypted file");
        let on_disk = fs::read(root.path().join("vault").join(&stored.encrypted_locator))
            .expect("read encrypted file");
        assert_eq!(&on_disk[..8], MAGIC);
        assert_eq!(&on_disk[8..12], &[VERSION, PURPOSE_FILE, 1, KDF_NONE]);
        assert_eq!(&on_disk[12..14], &0_u16.to_be_bytes());
        assert_eq!(&on_disk[14..16], &(NONCE_LEN as u16).to_be_bytes());
        assert_eq!(
            &on_disk[16..24],
            &(u64::try_from(plaintext.len()).expect("fixture size") + 16).to_be_bytes()
        );
        assert!(
            !on_disk
                .windows(plaintext.len())
                .any(|part| part == plaintext)
        );
        assert_eq!(
            vault
                .open_in_memory(&key, &stored.encrypted_locator)
                .expect("open encrypted file")
                .as_slice(),
            plaintext
        );
    }

    #[test]
    fn matches_the_accepted_architecture_portable_fixture() {
        let master_key = [0x11; KEY_LEN];
        let file_key = derive_file_key(&master_key).expect("derive fixture key");
        let envelope = seal_file_envelope_with_nonce(
            &file_key,
            &[0x22; NONCE_LEN],
            b"architecture-portable-envelope-fixture",
        )
        .expect("seal fixture");
        assert_eq!(
            hex_digest(&envelope),
            "3b46ed1d62b420d3dc6e9ba62ddfc52b15c5d921b923eed49e279a2f0da6b7da"
        );
    }

    #[test]
    fn matches_the_accepted_password_wrapper_fixtures() {
        let master_key = [0x55; KEY_LEN];
        let password = b"synthetic-vault-password";
        let fixtures = [
            (
                KdfProfile::Rfc9106LowMemoryV1,
                0x61,
                0x71,
                "d77cc533bf44270c08eeaaa64b05238b990d3156def3fd9cdd65a2dcccfd70e7",
            ),
            (
                KdfProfile::OwaspMinimumV1,
                0x62,
                0x72,
                "4fb57b26f766c5b0882ea54ff87075f722f55c2faf608407974a0f7f3f480f2d",
            ),
        ];
        for (profile, salt, nonce, expected_sha256) in fixtures {
            let wrapper = seal_password_wrapper_with_nonce(
                password,
                &master_key,
                profile,
                &[salt; 16],
                &[nonce; NONCE_LEN],
            )
            .expect("seal password wrapper fixture");
            assert_eq!(hex_digest(&wrapper), expected_sha256);
            assert_eq!(
                open_password_wrapper(&wrapper, password)
                    .expect("open password wrapper")
                    .as_slice(),
                master_key
            );
            assert!(open_password_wrapper(&wrapper, b"wrong-password").is_err());
        }
    }

    #[test]
    fn selects_the_kdf_profile_at_the_exact_unlock_budget_boundary() {
        assert_eq!(
            select_kdf_profile(Duration::from_millis(750)),
            KdfProfile::Rfc9106LowMemoryV1
        );
        assert_eq!(
            select_kdf_profile(Duration::from_millis(750) + Duration::from_nanos(1)),
            KdfProfile::OwaspMinimumV1
        );
    }

    #[test]
    fn rejects_a_password_wrapper_with_a_non_master_key_payload_length() {
        let mut wrapper = seal_password_wrapper_with_nonce(
            b"synthetic-vault-password",
            &[0x55; KEY_LEN],
            KdfProfile::OwaspMinimumV1,
            &[0x62; 16],
            &[0x72; NONCE_LEN],
        )
        .expect("seal password wrapper fixture");
        wrapper[16..24].copy_from_slice(&((KEY_LEN + 17) as u64).to_be_bytes());
        wrapper.push(0);

        assert!(password_wrapper_profile(&wrapper).is_err());
    }

    #[test]
    fn uses_fresh_nonces_and_rejects_tampering() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source = root.path().join("statement.pdf");
        fs::write(&source, b"same statement bytes").expect("write source fixture");
        let key = [0x51; KEY_LEN];
        let first_vault = FileVault::new(root.path().join("first"));
        let second_vault = FileVault::new(root.path().join("second"));
        let first = first_vault.store(&key, &source).expect("first store");
        let second = second_vault.store(&key, &source).expect("second store");
        assert!(first.created);
        assert!(second.created);
        let first_path = root.path().join("first").join(&first.encrypted_locator);
        let second_bytes = fs::read(root.path().join("second").join(&second.encrypted_locator))
            .expect("second encrypted file");
        let mut first_bytes = fs::read(&first_path).expect("first encrypted file");
        assert_ne!(first_bytes, second_bytes);

        let last = first_bytes.last_mut().expect("encrypted file is not empty");
        *last ^= 1;
        fs::write(&first_path, first_bytes).expect("tamper fixture");
        assert!(
            first_vault
                .open_in_memory(&key, &first.encrypted_locator)
                .is_err()
        );
    }

    #[test]
    fn rejects_locators_outside_the_vault_files_directory() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let vault = FileVault::new(root.path());
        assert!(vault.open_in_memory(&[0x61; KEY_LEN], "../secret").is_err());
        assert!(
            vault
                .open_in_memory(&[0x61; KEY_LEN], "files/nested/secret")
                .is_err()
        );
    }

    #[test]
    fn rejects_an_envelope_with_an_overflowing_ciphertext_length() {
        let mut envelope = vec![0_u8; HEADER_FIXED_LEN + NONCE_LEN + 16];
        envelope[..8].copy_from_slice(MAGIC);
        envelope[8..12].copy_from_slice(&[VERSION, PURPOSE_FILE, 1, KDF_NONE]);
        envelope[14..16].copy_from_slice(&(NONCE_LEN as u16).to_be_bytes());
        envelope[16..24].copy_from_slice(&u64::MAX.to_be_bytes());

        assert!(open_file_envelope(&[0x71; KEY_LEN], &envelope).is_err());
    }

    #[test]
    fn reuses_a_verified_envelope_and_removes_only_unreferenced_files() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source = root.path().join("statement.pdf");
        fs::write(&source, b"statement evidence").expect("write source fixture");
        let key = [0x81; KEY_LEN];
        let vault = FileVault::new(root.path().join("vault"));
        let first = vault.store(&key, &source).expect("first store");
        let second = vault.store(&key, &source).expect("second store");
        assert!(!second.created);

        vault
            .remove_unreferenced(&HashSet::from([first.encrypted_locator.clone()]))
            .expect("keep referenced file");
        assert!(
            root.path()
                .join("vault")
                .join(&first.encrypted_locator)
                .exists()
        );
        vault
            .remove_unreferenced(&HashSet::new())
            .expect("remove orphaned file");
        assert!(
            !root
                .path()
                .join("vault")
                .join(&first.encrypted_locator)
                .exists()
        );
    }
}
