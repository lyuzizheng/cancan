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
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"CCENV001";
const VERSION: u8 = 1;
const PURPOSE_FILE: u8 = 1;
const ALGORITHM_XCHACHA20_POLY1305: u8 = 1;
const KDF_NONE: u8 = 0;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;
const HEADER_LEN: usize = 24 + NONCE_LEN;
const FILE_KEY_CONTEXT: &[u8] = b"cancan:file:v1";

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StoredFile {
    pub(crate) byte_size: u64,
    pub(crate) created: bool,
    pub(crate) encrypted_locator: String,
    pub(crate) file_sha256: String,
}

#[derive(Debug)]
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

    #[cfg(test)]
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

    pub(crate) fn exists(&self, encrypted_locator: &str) -> io::Result<bool> {
        Ok(self.resolve_locator(encrypted_locator)?.is_file())
    }

    pub(crate) fn remove(&self, encrypted_locator: &str) -> io::Result<()> {
        let path = self.resolve_locator(encrypted_locator)?;
        if path.exists() {
            fs::remove_file(path)?;
            let directory = self.root.join("files");
            File::open(directory)?.sync_all()?;
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
    seal_file_envelope_with_nonce(key, &nonce, plaintext)
}

fn seal_file_envelope_with_nonce(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
) -> io::Result<Vec<u8>> {
    let ciphertext_len = plaintext
        .len()
        .checked_add(16)
        .ok_or_else(|| io::Error::other("ciphertext length overflow"))?;
    let mut header = Vec::with_capacity(HEADER_LEN + ciphertext_len);
    header.extend_from_slice(MAGIC);
    header.push(VERSION);
    header.push(PURPOSE_FILE);
    header.push(ALGORITHM_XCHACHA20_POLY1305);
    header.push(KDF_NONE);
    header.extend_from_slice(&0_u16.to_be_bytes());
    header.extend_from_slice(&(NONCE_LEN as u16).to_be_bytes());
    header.extend_from_slice(&(ciphertext_len as u64).to_be_bytes());
    header.extend_from_slice(nonce);

    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::other("invalid file encryption key"))?;
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad: &header,
            },
        )
        .map_err(|_| io::Error::other("file encryption failed"))?;
    header.extend_from_slice(&ciphertext);
    Ok(header)
}

fn open_file_envelope(key: &[u8; KEY_LEN], envelope: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    if envelope.len() < HEADER_LEN + 16
        || &envelope[..8] != MAGIC
        || envelope[8] != VERSION
        || envelope[9] != PURPOSE_FILE
        || envelope[10] != ALGORITHM_XCHACHA20_POLY1305
        || envelope[11] != KDF_NONE
        || u16::from_be_bytes([envelope[12], envelope[13]]) != 0
        || usize::from(u16::from_be_bytes([envelope[14], envelope[15]])) != NONCE_LEN
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid file envelope header",
        ));
    }
    let ciphertext_len = usize::try_from(u64::from_be_bytes(
        envelope[16..24]
            .try_into()
            .expect("fixed envelope header length"),
    ))
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "ciphertext length overflow"))?;
    let expected_len = HEADER_LEN
        .checked_add(ciphertext_len)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "envelope length overflow"))?;
    if envelope.len() != expected_len || ciphertext_len < 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid file envelope size",
        ));
    }

    let nonce = &envelope[24..HEADER_LEN];
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::other("invalid file decryption key"))?;
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: &envelope[HEADER_LEN..],
                aad: &envelope[..HEADER_LEN],
            },
        )
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file authentication failed"))?;
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
        let mut envelope = vec![0_u8; HEADER_LEN + 16];
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
