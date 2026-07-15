use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    env,
    error::Error,
    ffi::c_void,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    time::Instant,
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"CCENV001";
const VERSION: u8 = 1;
const ALGORITHM_XCHACHA20_POLY1305: u8 = 1;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;
const HEADER_FIXED_LEN: usize = 24;

type ValidationResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum EnvelopePurpose {
    File = 1,
    PasswordWrapper = 2,
    RecoveryWrapper = 3,
    Backup = 4,
}

impl TryFrom<u8> for EnvelopePurpose {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::File),
            2 => Ok(Self::PasswordWrapper),
            3 => Ok(Self::RecoveryWrapper),
            4 => Ok(Self::Backup),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unknown envelope purpose",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum KdfProfile {
    None = 0,
    Rfc9106LowMemoryV1 = 1,
    OwaspMinimumV1 = 2,
}

impl KdfProfile {
    fn params(self) -> ValidationResult<Params> {
        let params = match self {
            Self::Rfc9106LowMemoryV1 => Params::new(65_536, 3, 4, Some(KEY_LEN)),
            Self::OwaspMinimumV1 => Params::new(19_456, 2, 1, Some(KEY_LEN)),
            Self::None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "envelope has no KDF profile",
                )
                .into());
            }
        };
        params.map_err(|error| io::Error::other(format!("Argon2 params failed: {error}")).into())
    }
}

impl TryFrom<u8> for KdfProfile {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Rfc9106LowMemoryV1),
            2 => Ok(Self::OwaspMinimumV1),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unknown KDF profile",
            )),
        }
    }
}

#[derive(Debug)]
struct ParsedEnvelope<'a> {
    purpose: EnvelopePurpose,
    profile: KdfProfile,
    salt: &'a [u8],
    nonce: &'a [u8],
    ciphertext: &'a [u8],
    authenticated_header: &'a [u8],
}

#[derive(Debug, Serialize)]
struct EvidenceReport {
    primary_kdf_ms: u128,
    fallback_kdf_ms: u128,
    selected_profile: &'static str,
    envelope_sha256: String,
    rendered_pixels_sha256: String,
    deletion_crash_cases: usize,
    restore_crash_cases: usize,
    keychain: &'static str,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SourceState {
    file_state: FileState,
    audit_events: Vec<String>,
    record_count: usize,
    ledger_event_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct RestoreManifest {
    vault_version: u32,
    schema_version: u32,
    database_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FileState {
    Available,
    Deleted,
    Missing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeleteCrashPoint {
    BeforeDecision,
    AfterDecision,
    AfterBlobRemoval,
    Completed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RestoreCrashPoint {
    AfterCandidateFilesSync,
    AfterCandidateDirectorySync,
    AfterCandidateValidation,
    AfterLocatorTempSync,
    AfterLocatorSwitch,
    Completed,
}

fn main() -> ValidationResult<()> {
    match env::args().nth(1).as_deref() {
        Some("evidence") => {
            let report = run_evidence()?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Some(command) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command: {command}"),
        )
        .into()),
        None => Err(io::Error::new(io::ErrorKind::InvalidInput, "usage: evidence").into()),
    }
}

fn run_evidence() -> ValidationResult<EvidenceReport> {
    let (primary_kdf_ms, fallback_kdf_ms, selected_profile) = benchmark_kdf_profiles()?;
    let envelope = deterministic_envelope_fixture()?;
    verify_envelope_contract(&envelope)?;
    verify_wrapper_contract()?;
    let rendered_pixels = verify_in_memory_pdf_render()?;
    verify_money_source_keychain_password()?;
    verify_delete_crash_matrix()?;
    verify_restore_crash_matrix()?;

    Ok(EvidenceReport {
        primary_kdf_ms,
        fallback_kdf_ms,
        selected_profile: match selected_profile {
            KdfProfile::Rfc9106LowMemoryV1 => "rfc9106-low-memory-v1",
            KdfProfile::OwaspMinimumV1 => "owasp-minimum-v1",
            KdfProfile::None => unreachable!(),
        },
        envelope_sha256: sha256_hex(&envelope),
        rendered_pixels_sha256: sha256_hex(&rendered_pixels),
        deletion_crash_cases: 4,
        restore_crash_cases: 6,
        keychain: "write-update-read-delete passed",
    })
}

fn derive_password_key(
    password: &[u8],
    salt: &[u8],
    profile: KdfProfile,
) -> ValidationResult<Zeroizing<[u8; KEY_LEN]>> {
    if salt.len() != 16 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "salt must be 16 bytes").into());
    }
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, profile.params()?);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    argon2
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|error| io::Error::other(format!("Argon2 derivation failed: {error}")))?;
    Ok(key)
}

fn benchmark_kdf_profiles() -> ValidationResult<(u128, u128, KdfProfile)> {
    let password = b"synthetic-vault-password";
    let salt = [0x5a; 16];

    let started = Instant::now();
    let primary = derive_password_key(password, &salt, KdfProfile::Rfc9106LowMemoryV1)?;
    let primary_ms = started.elapsed().as_millis();

    let started = Instant::now();
    let fallback = derive_password_key(password, &salt, KdfProfile::OwaspMinimumV1)?;
    let fallback_ms = started.elapsed().as_millis();

    if primary.as_ref() == fallback.as_ref() {
        return Err(io::Error::other("distinct KDF profiles produced the same key").into());
    }
    let selected = if primary_ms <= 750 {
        KdfProfile::Rfc9106LowMemoryV1
    } else {
        KdfProfile::OwaspMinimumV1
    };
    Ok((primary_ms, fallback_ms, selected))
}

fn derive_subkey(master_key: &[u8; KEY_LEN], context: &[u8]) -> ValidationResult<[u8; KEY_LEN]> {
    let hkdf = Hkdf::<Sha256>::new(None, master_key);
    let mut key = [0_u8; KEY_LEN];
    hkdf.expand(context, &mut key)
        .map_err(|_| io::Error::other("HKDF expansion failed"))?;
    Ok(key)
}

fn seal_envelope(
    purpose: EnvelopePurpose,
    profile: KdfProfile,
    salt: &[u8],
    key: &[u8; KEY_LEN],
    plaintext: &[u8],
) -> ValidationResult<Vec<u8>> {
    let mut nonce = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    seal_envelope_with_nonce(purpose, profile, salt, key, &nonce, plaintext)
}

fn seal_envelope_with_nonce(
    purpose: EnvelopePurpose,
    profile: KdfProfile,
    salt: &[u8],
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
) -> ValidationResult<Vec<u8>> {
    if profile == KdfProfile::None && !salt.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "non-KDF envelope must not contain a salt",
        )
        .into());
    }
    if profile != KdfProfile::None && salt.len() != 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "KDF envelope must contain a 16-byte salt",
        )
        .into());
    }

    let ciphertext_len = plaintext
        .len()
        .checked_add(16)
        .ok_or_else(|| io::Error::other("ciphertext length overflow"))?;
    let mut header = Vec::with_capacity(HEADER_FIXED_LEN + salt.len() + NONCE_LEN);
    header.extend_from_slice(MAGIC);
    header.push(VERSION);
    header.push(purpose as u8);
    header.push(ALGORITHM_XCHACHA20_POLY1305);
    header.push(profile as u8);
    header.extend_from_slice(&(salt.len() as u16).to_be_bytes());
    header.extend_from_slice(&(NONCE_LEN as u16).to_be_bytes());
    header.extend_from_slice(&(ciphertext_len as u64).to_be_bytes());
    header.extend_from_slice(salt);
    header.extend_from_slice(nonce);

    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::other("invalid encryption key"))?;
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad: &header,
            },
        )
        .map_err(|_| io::Error::other("encryption failed"))?;
    debug_assert_eq!(ciphertext.len(), ciphertext_len);
    header.extend_from_slice(&ciphertext);
    Ok(header)
}

fn parse_envelope(bytes: &[u8]) -> ValidationResult<ParsedEnvelope<'_>> {
    if bytes.len() < HEADER_FIXED_LEN || &bytes[..MAGIC.len()] != MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid envelope magic").into());
    }
    if bytes[8] != VERSION || bytes[10] != ALGORITHM_XCHACHA20_POLY1305 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported envelope version or algorithm",
        )
        .into());
    }
    let purpose = EnvelopePurpose::try_from(bytes[9])?;
    let profile = KdfProfile::try_from(bytes[11])?;
    let salt_len = u16::from_be_bytes([bytes[12], bytes[13]]) as usize;
    let nonce_len = u16::from_be_bytes([bytes[14], bytes[15]]) as usize;
    let ciphertext_len = usize::try_from(u64::from_be_bytes(
        bytes[16..24].try_into().expect("fixed envelope header"),
    ))
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "ciphertext length overflow"))?;
    if nonce_len != NONCE_LEN || salt_len > 16 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid envelope lengths").into());
    }
    if (profile == KdfProfile::None && salt_len != 0)
        || (profile != KdfProfile::None && salt_len != 16)
    {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid KDF salt length").into());
    }
    let header_len = HEADER_FIXED_LEN
        .checked_add(salt_len)
        .and_then(|length| length.checked_add(nonce_len))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "header length overflow"))?;
    let total_len = header_len
        .checked_add(ciphertext_len)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "envelope length overflow"))?;
    if bytes.len() != total_len || ciphertext_len < 16 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid envelope size").into());
    }
    Ok(ParsedEnvelope {
        purpose,
        profile,
        salt: &bytes[HEADER_FIXED_LEN..HEADER_FIXED_LEN + salt_len],
        nonce: &bytes[HEADER_FIXED_LEN + salt_len..header_len],
        ciphertext: &bytes[header_len..],
        authenticated_header: &bytes[..header_len],
    })
}

fn open_envelope(
    bytes: &[u8],
    expected_purpose: EnvelopePurpose,
    key: &[u8; KEY_LEN],
) -> ValidationResult<Zeroizing<Vec<u8>>> {
    let envelope = parse_envelope(bytes)?;
    if envelope.purpose != expected_purpose {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "wrong envelope purpose").into());
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::other("invalid decryption key"))?;
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(envelope.nonce),
            Payload {
                msg: envelope.ciphertext,
                aad: envelope.authenticated_header,
            },
        )
        .map_err(|_| io::Error::other("decryption or authentication failed"))?;
    Ok(Zeroizing::new(plaintext))
}

fn open_password_wrapper(bytes: &[u8], password: &[u8]) -> ValidationResult<Zeroizing<Vec<u8>>> {
    let envelope = parse_envelope(bytes)?;
    if envelope.purpose != EnvelopePurpose::PasswordWrapper || envelope.profile == KdfProfile::None
    {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid password wrapper").into());
    }
    let key = derive_password_key(password, envelope.salt, envelope.profile)?;
    open_envelope(bytes, EnvelopePurpose::PasswordWrapper, &key)
}

fn deterministic_envelope_fixture() -> ValidationResult<Vec<u8>> {
    let master_key = [0x11; KEY_LEN];
    let file_key = derive_subkey(&master_key, b"cancan:file:v1")?;
    seal_envelope_with_nonce(
        EnvelopePurpose::File,
        KdfProfile::None,
        &[],
        &file_key,
        &[0x22; NONCE_LEN],
        b"architecture-portable-envelope-fixture",
    )
}

fn verify_envelope_contract(envelope: &[u8]) -> ValidationResult<()> {
    let master_key = [0x11; KEY_LEN];
    let file_key = derive_subkey(&master_key, b"cancan:file:v1")?;
    let backup_key = derive_subkey(&master_key, b"cancan:backup:v1")?;
    if file_key == backup_key {
        return Err(io::Error::other("file and backup subkeys are not separated").into());
    }
    let plaintext = open_envelope(envelope, EnvelopePurpose::File, &file_key)?;
    if plaintext.as_slice() != b"architecture-portable-envelope-fixture" {
        return Err(io::Error::other("fixture plaintext mismatch").into());
    }
    if open_envelope(envelope, EnvelopePurpose::File, &backup_key).is_ok() {
        return Err(io::Error::other("wrong purpose key opened file envelope").into());
    }
    for index in [8_usize, 9, 10, 11, envelope.len() - 1] {
        let mut tampered = envelope.to_vec();
        tampered[index] ^= 1;
        if open_envelope(&tampered, EnvelopePurpose::File, &file_key).is_ok() {
            return Err(
                io::Error::other(format!("tampered envelope byte {index} authenticated")).into(),
            );
        }
    }
    let first = seal_envelope(
        EnvelopePurpose::File,
        KdfProfile::None,
        &[],
        &file_key,
        b"same plaintext",
    )?;
    let second = seal_envelope(
        EnvelopePurpose::File,
        KdfProfile::None,
        &[],
        &file_key,
        b"same plaintext",
    )?;
    if first == second
        || open_envelope(&first, EnvelopePurpose::File, &file_key)?.as_slice() != b"same plaintext"
        || open_envelope(&second, EnvelopePurpose::File, &file_key)?.as_slice() != b"same plaintext"
    {
        return Err(io::Error::other("normal sealing did not generate fresh nonces").into());
    }
    Ok(())
}

fn verify_wrapper_contract() -> ValidationResult<()> {
    let master_key = [0x55; KEY_LEN];
    let password = b"synthetic-vault-password";
    let mut fixture_hashes = Vec::new();
    for (profile, salt_byte, nonce_byte) in [
        (KdfProfile::Rfc9106LowMemoryV1, 0x61, 0x71),
        (KdfProfile::OwaspMinimumV1, 0x62, 0x72),
    ] {
        let salt = [salt_byte; 16];
        let password_key = derive_password_key(password, &salt, profile)?;
        let password_wrapper = seal_envelope_with_nonce(
            EnvelopePurpose::PasswordWrapper,
            profile,
            &salt,
            &password_key,
            &[nonce_byte; NONCE_LEN],
            &master_key,
        )?;
        if open_password_wrapper(&password_wrapper, password)?.as_slice() != master_key {
            return Err(io::Error::other("password wrapper did not recover the master key").into());
        }
        if open_password_wrapper(&password_wrapper, b"wrong-password").is_ok() {
            return Err(io::Error::other("password wrapper accepted the wrong password").into());
        }
        fixture_hashes.push(sha256_hex(&password_wrapper));
    }

    let recovery_key = [0x81; KEY_LEN];
    let recovery_wrapper = seal_envelope_with_nonce(
        EnvelopePurpose::RecoveryWrapper,
        KdfProfile::None,
        &[],
        &recovery_key,
        &[0x82; NONCE_LEN],
        &master_key,
    )?;
    if open_envelope(
        &recovery_wrapper,
        EnvelopePurpose::RecoveryWrapper,
        &recovery_key,
    )?
    .as_slice()
        != master_key
    {
        return Err(io::Error::other("recovery wrapper did not recover the master key").into());
    }
    let wrong_recovery_key = [0x99; KEY_LEN];
    if open_envelope(
        &recovery_wrapper,
        EnvelopePurpose::RecoveryWrapper,
        &wrong_recovery_key,
    )
    .is_ok()
    {
        return Err(io::Error::other("recovery wrapper accepted the wrong key").into());
    }
    fixture_hashes.push(sha256_hex(&recovery_wrapper));
    let expected = [
        "d77cc533bf44270c08eeaaa64b05238b990d3156def3fd9cdd65a2dcccfd70e7",
        "4fb57b26f766c5b0882ea54ff87075f722f55c2faf608407974a0f7f3f480f2d",
        "7a2edcc69e464bbf25928609659f8ada3ab0db7f96a15b5cc67dce85c2bcc4c4",
    ];
    if fixture_hashes != expected {
        return Err(io::Error::other(format!(
            "wrapper compatibility fixture hashes: {fixture_hashes:?}"
        ))
        .into());
    }
    Ok(())
}

struct KeychainCleanup<'a>(&'a keyring::Entry);

impl Drop for KeychainCleanup<'_> {
    fn drop(&mut self) {
        let _ = self.0.delete_credential();
    }
}

fn verify_money_source_keychain_password() -> ValidationResult<()> {
    let username = format!("money-source-{}", std::process::id());
    let entry = keyring::Entry::new("app.cancan.statement-password.validation", &username)?;
    let _ = entry.delete_credential();
    let _cleanup = KeychainCleanup(&entry);
    entry.set_secret(b"first-synthetic-password")?;
    entry.set_secret(b"updated-synthetic-password")?;
    let loaded = Zeroizing::new(entry.get_secret()?);
    if loaded.as_slice() != b"updated-synthetic-password" {
        return Err(io::Error::other("Keychain did not replace the source password").into());
    }
    entry.delete_credential()?;
    if entry.get_secret().is_ok() {
        return Err(io::Error::other("Keychain source password survived deletion").into());
    }
    Ok(())
}

fn verify_in_memory_pdf_render() -> ValidationResult<Vec<u8>> {
    let root = tempfile::tempdir()?;
    let master_key = [0x33; KEY_LEN];
    let file_key = derive_subkey(&master_key, b"cancan:file:v1")?;
    let pdf = synthetic_pdf();
    let envelope = seal_envelope(
        EnvelopePurpose::File,
        KdfProfile::None,
        &[],
        &file_key,
        &pdf,
    )?;
    let encrypted_path = root.path().join("statement.cancan-encrypted");
    fs::write(&encrypted_path, &envelope)?;
    if fs::read(&encrypted_path)?
        .windows(5)
        .any(|window| window == b"%PDF-")
    {
        return Err(io::Error::other("plaintext PDF signature reached disk").into());
    }
    let before = directory_entries(root.path())?;
    let decrypted = open_envelope(&envelope, EnvelopePurpose::File, &file_key)?;
    let pixels = render_pdf_page_in_memory(&decrypted)?;
    let after = directory_entries(root.path())?;
    if before != after || after != vec![PathBuf::from("statement.cancan-encrypted")] {
        return Err(io::Error::other("PDF rendering created an unexpected file").into());
    }
    if pixels.iter().all(|byte| *byte == 0xff) {
        return Err(io::Error::other("PDF renderer did not change the pixel buffer").into());
    }
    Ok(pixels)
}

fn synthetic_pdf() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 64 64] /Resources << >> /Contents 4 0 R >>",
        "<< /Length 23 >>\nstream\n0 0 0 rg 0 0 64 64 re f\nendstream",
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        write!(&mut pdf, "{} 0 obj\n{}\nendobj\n", index + 1, object).expect("write PDF");
    }
    let xref = pdf.len();
    write!(
        &mut pdf,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    )
    .expect("write xref");
    for offset in offsets {
        writeln!(&mut pdf, "{offset:010} 00000 n ").expect("write xref entry");
    }
    write!(
        &mut pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("write trailer");
    pdf
}

fn directory_entries(root: &Path) -> ValidationResult<Vec<PathBuf>> {
    let mut entries = fs::read_dir(root)?
        .map(|entry| entry.map(|value| PathBuf::from(value.file_name())))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    Ok(entries)
}

#[cfg(target_os = "macos")]
fn render_pdf_page_in_memory(pdf: &[u8]) -> ValidationResult<Vec<u8>> {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 64;
    const BYTES_PER_ROW: usize = WIDTH * 4;
    const ALPHA_PREMULTIPLIED_LAST: u32 = 1;

    let provider = unsafe {
        CGDataProviderCreateWithData(std::ptr::null_mut(), pdf.as_ptr().cast(), pdf.len(), None)
    };
    if provider.is_null() {
        return Err(io::Error::other("Core Graphics data provider failed").into());
    }
    let document = unsafe { CGPDFDocumentCreateWithProvider(provider) };
    if document.is_null() {
        unsafe { CGDataProviderRelease(provider) };
        return Err(io::Error::other("Core Graphics rejected the in-memory PDF").into());
    }
    if unsafe { CGPDFDocumentGetNumberOfPages(document) } != 1 {
        unsafe {
            CGPDFDocumentRelease(document);
            CGDataProviderRelease(provider);
        }
        return Err(io::Error::other("synthetic PDF page count mismatch").into());
    }
    let page = unsafe { CGPDFDocumentGetPage(document, 1) };
    let color_space = unsafe { CGColorSpaceCreateDeviceRGB() };
    if page.is_null() || color_space.is_null() {
        unsafe {
            if !color_space.is_null() {
                CGColorSpaceRelease(color_space);
            }
            CGPDFDocumentRelease(document);
            CGDataProviderRelease(provider);
        }
        return Err(io::Error::other("Core Graphics PDF page or color space failed").into());
    }
    let mut pixels = vec![0xff_u8; HEIGHT * BYTES_PER_ROW];
    let context = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast(),
            WIDTH,
            HEIGHT,
            8,
            BYTES_PER_ROW,
            color_space,
            ALPHA_PREMULTIPLIED_LAST,
        )
    };
    if context.is_null() {
        unsafe {
            CGColorSpaceRelease(color_space);
            CGPDFDocumentRelease(document);
            CGDataProviderRelease(provider);
        }
        return Err(io::Error::other("Core Graphics bitmap context failed").into());
    }
    unsafe {
        CGContextDrawPDFPage(context, page);
        CGContextRelease(context);
        CGColorSpaceRelease(color_space);
        CGPDFDocumentRelease(document);
        CGDataProviderRelease(provider);
    }
    Ok(pixels)
}

#[cfg(not(target_os = "macos"))]
fn render_pdf_page_in_memory(_pdf: &[u8]) -> ValidationResult<Vec<u8>> {
    Err(io::Error::other("the Phase 1 viewer evidence requires macOS").into())
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGDataProviderCreateWithData(
        info: *mut c_void,
        data: *const c_void,
        size: usize,
        release_data: Option<unsafe extern "C" fn(*mut c_void, *const c_void, usize)>,
    ) -> *mut c_void;
    fn CGDataProviderRelease(provider: *mut c_void);
    fn CGPDFDocumentCreateWithProvider(provider: *mut c_void) -> *mut c_void;
    fn CGPDFDocumentRelease(document: *mut c_void);
    fn CGPDFDocumentGetNumberOfPages(document: *mut c_void) -> usize;
    fn CGPDFDocumentGetPage(document: *mut c_void, page: usize) -> *mut c_void;
    fn CGColorSpaceCreateDeviceRGB() -> *mut c_void;
    fn CGColorSpaceRelease(color_space: *mut c_void);
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        color_space: *mut c_void,
        bitmap_info: u32,
    ) -> *mut c_void;
    fn CGContextDrawPDFPage(context: *mut c_void, page: *mut c_void);
    fn CGContextRelease(context: *mut c_void);
}

fn initialize_source(root: &Path) -> ValidationResult<()> {
    fs::create_dir_all(root)?;
    fs::write(root.join("source.enc"), b"synthetic encrypted blob")?;
    write_json_atomically(
        &root.join("source.json"),
        &SourceState {
            file_state: FileState::Available,
            audit_events: Vec::new(),
            record_count: 6,
            ledger_event_count: 1,
        },
    )
}

fn delete_source(root: &Path, crash: DeleteCrashPoint) -> ValidationResult<()> {
    if crash == DeleteCrashPoint::BeforeDecision {
        return Ok(());
    }
    let state_path = root.join("source.json");
    let mut state: SourceState = serde_json::from_slice(&fs::read(&state_path)?)?;
    if state.file_state != FileState::Deleted {
        state.file_state = FileState::Deleted;
        state.audit_events.push("source_file_deleted".to_string());
        write_json_atomically(&state_path, &state)?;
    }
    if crash == DeleteCrashPoint::AfterDecision {
        return Ok(());
    }
    let blob = root.join("source.enc");
    match fs::remove_file(&blob) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    if crash == DeleteCrashPoint::AfterBlobRemoval {
        return Ok(());
    }
    recover_source_deletion(root)
}

fn recover_source_deletion(root: &Path) -> ValidationResult<()> {
    let state_path = root.join("source.json");
    let mut state: SourceState = serde_json::from_slice(&fs::read(&state_path)?)?;
    let blob = root.join("source.enc");
    match state.file_state {
        FileState::Deleted => match fs::remove_file(blob) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        },
        FileState::Available if !blob.exists() => {
            state.file_state = FileState::Missing;
            write_json_atomically(&state_path, &state)?;
        }
        FileState::Available | FileState::Missing => {}
    }
    Ok(())
}

fn verify_delete_crash_matrix() -> ValidationResult<()> {
    for crash in [
        DeleteCrashPoint::BeforeDecision,
        DeleteCrashPoint::AfterDecision,
        DeleteCrashPoint::AfterBlobRemoval,
        DeleteCrashPoint::Completed,
    ] {
        let root = tempfile::tempdir()?;
        initialize_source(root.path())?;
        delete_source(root.path(), crash)?;
        recover_source_deletion(root.path())?;
        let state: SourceState =
            serde_json::from_slice(&fs::read(root.path().join("source.json"))?)?;
        if crash == DeleteCrashPoint::BeforeDecision {
            if state.file_state != FileState::Available
                || !root.path().join("source.enc").exists()
                || !state.audit_events.is_empty()
            {
                return Err(io::Error::other("pre-decision crash changed source state").into());
            }
        } else if state.file_state != FileState::Deleted
            || root.path().join("source.enc").exists()
            || state.audit_events != ["source_file_deleted"]
            || state.record_count != 6
            || state.ledger_event_count != 1
        {
            return Err(io::Error::other(format!(
                "delete recovery did not converge after {crash:?}"
            ))
            .into());
        }
        recover_source_deletion(root.path())?;
    }
    Ok(())
}

fn initialize_restore_root(root: &Path) -> ValidationResult<()> {
    fs::create_dir_all(root.join("vault-a"))?;
    write_and_sync(&root.join("vault-a/finance.sqlite"), b"active-vault-a")?;
    File::open(root.join("vault-a"))?.sync_all()?;
    write_and_sync(&root.join("active-vault"), b"vault-a")?;
    File::open(root)?.sync_all()?;
    Ok(())
}

fn restore_vault(root: &Path, crash: RestoreCrashPoint) -> ValidationResult<()> {
    let candidate = root.join("vault-b");
    fs::create_dir_all(&candidate)?;
    File::open(root)?.sync_all()?;
    let database = b"restored-vault-b";
    write_and_sync(&candidate.join("finance.sqlite"), database)?;
    write_and_sync(
        &candidate.join("manifest.json"),
        &serde_json::to_vec(&RestoreManifest {
            vault_version: 1,
            schema_version: 1,
            database_sha256: sha256_hex(database),
        })?,
    )?;
    if crash == RestoreCrashPoint::AfterCandidateFilesSync {
        return Ok(());
    }
    File::open(&candidate)?.sync_all()?;
    if crash == RestoreCrashPoint::AfterCandidateDirectorySync {
        return Ok(());
    }
    validate_candidate(&candidate)?;
    if crash == RestoreCrashPoint::AfterCandidateValidation {
        return Ok(());
    }
    let locator_temp = root.join("active-vault.tmp");
    write_and_sync(&locator_temp, b"vault-b")?;
    if crash == RestoreCrashPoint::AfterLocatorTempSync {
        return Ok(());
    }
    fs::rename(&locator_temp, root.join("active-vault"))?;
    File::open(root)?.sync_all()?;
    if crash == RestoreCrashPoint::AfterLocatorSwitch {
        return Ok(());
    }
    recover_restore(root)
}

fn validate_candidate(candidate: &Path) -> ValidationResult<()> {
    let manifest: RestoreManifest =
        serde_json::from_slice(&fs::read(candidate.join("manifest.json"))?)?;
    let database = fs::read(candidate.join("finance.sqlite"))?;
    if manifest.vault_version != 1
        || manifest.schema_version != 1
        || manifest.database_sha256 != sha256_hex(&database)
        || database != b"restored-vault-b"
    {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid restore candidate").into());
    }
    Ok(())
}

fn recover_restore(root: &Path) -> ValidationResult<()> {
    let locator = fs::read_to_string(root.join("active-vault"))?;
    match locator.trim() {
        "vault-a" => {
            if fs::read(root.join("vault-a/finance.sqlite"))? != b"active-vault-a" {
                return Err(io::Error::other("existing active Vault failed validation").into());
            }
        }
        "vault-b" => validate_candidate(&root.join("vault-b"))?,
        _ => return Err(io::Error::other("active Vault locator is invalid").into()),
    }
    let temporary = root.join("active-vault.tmp");
    if temporary.exists() {
        fs::remove_file(temporary)?;
    }
    Ok(())
}

fn verify_restore_crash_matrix() -> ValidationResult<()> {
    for crash in [
        RestoreCrashPoint::AfterCandidateFilesSync,
        RestoreCrashPoint::AfterCandidateDirectorySync,
        RestoreCrashPoint::AfterCandidateValidation,
        RestoreCrashPoint::AfterLocatorTempSync,
        RestoreCrashPoint::AfterLocatorSwitch,
        RestoreCrashPoint::Completed,
    ] {
        let root = tempfile::tempdir()?;
        initialize_restore_root(root.path())?;
        restore_vault(root.path(), crash)?;
        let active = fs::read_to_string(root.path().join("active-vault"))?;
        if matches!(
            crash,
            RestoreCrashPoint::AfterLocatorSwitch | RestoreCrashPoint::Completed
        ) {
            if active != "vault-b"
                || fs::read(root.path().join("vault-b/finance.sqlite"))? != b"restored-vault-b"
            {
                return Err(
                    io::Error::other("atomic restore switch did not select candidate").into(),
                );
            }
        } else if active != "vault-a"
            || fs::read(root.path().join("vault-a/finance.sqlite"))? != b"active-vault-a"
        {
            return Err(io::Error::other(format!(
                "restore crash changed active Vault after {crash:?}"
            ))
            .into());
        }
        recover_restore(root.path())?;
        if root.path().join("active-vault.tmp").exists() {
            return Err(io::Error::other("restore recovery left a temporary locator").into());
        }
    }
    Ok(())
}

fn write_json_atomically(path: &Path, value: &impl Serialize) -> ValidationResult<()> {
    let bytes = serde_json::to_vec(value)?;
    let temporary = path.with_extension("tmp");
    write_and_sync(&temporary, &bytes)?;
    fs::rename(&temporary, path)?;
    File::open(
        path.parent()
            .ok_or_else(|| io::Error::other("missing parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> ValidationResult<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_kdf_profiles_derive_distinct_keys_and_select_by_budget() {
        let (primary_ms, fallback_ms, selected) = benchmark_kdf_profiles().expect("KDF profiles");
        assert!(primary_ms > 0);
        assert!(fallback_ms > 0);
        assert_eq!(
            selected,
            if primary_ms <= 750 {
                KdfProfile::Rfc9106LowMemoryV1
            } else {
                KdfProfile::OwaspMinimumV1
            }
        );
    }

    #[test]
    fn envelope_is_deterministic_portable_and_authenticated() {
        let envelope = deterministic_envelope_fixture().expect("fixture");
        verify_envelope_contract(&envelope).expect("envelope contract");
        assert_eq!(
            sha256_hex(&envelope),
            "3b46ed1d62b420d3dc6e9ba62ddfc52b15c5d921b923eed49e279a2f0da6b7da"
        );
    }

    #[test]
    fn password_and_recovery_wrappers_reject_wrong_credentials() {
        verify_wrapper_contract().expect("wrapper contract");
    }

    #[test]
    fn encrypted_pdf_renders_without_plaintext_file() {
        let pixels = verify_in_memory_pdf_render().expect("in-memory render");
        assert_eq!(pixels.len(), 64 * 64 * 4);
    }

    #[test]
    fn source_deletion_converges_across_crash_matrix() {
        verify_delete_crash_matrix().expect("delete crash matrix");
    }

    #[test]
    fn restore_switches_only_after_candidate_validation() {
        verify_restore_crash_matrix().expect("restore crash matrix");
    }

    #[test]
    fn invalid_restore_candidate_never_changes_active_locator() {
        let root = tempfile::tempdir().expect("restore root");
        initialize_restore_root(root.path()).expect("active Vault");
        let candidate = root.path().join("vault-b");
        fs::create_dir_all(&candidate).expect("candidate directory");
        fs::write(candidate.join("finance.sqlite"), b"tampered").expect("candidate database");
        fs::write(candidate.join("manifest.json"), b"{\"vault_version\":1}")
            .expect("candidate manifest");
        assert!(validate_candidate(&candidate).is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("active-vault")).expect("active locator"),
            "vault-a"
        );
    }
}
