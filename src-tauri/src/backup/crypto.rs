//! Passphrase-derived encryption for backup archives.
//!
//! Container format (all header fields unencrypted, header doubles as AEAD
//! associated data so it cannot be tampered with):
//!
//! ```text
//! bytes 0..4    magic  b"GRNT"
//! byte  4       format version (1)
//! bytes 5..9    Argon2 memory cost in KiB (u32 LE)
//! bytes 9..13   Argon2 iterations        (u32 LE)
//! bytes 13..17  Argon2 parallelism       (u32 LE)
//! bytes 17..33  Argon2 salt (16 bytes)
//! bytes 33..57  XChaCha20 nonce (24 bytes, random per backup)
//! bytes 57..    XChaCha20-Poly1305 ciphertext
//! ```
//!
//! The derived key is cached in `.granit/backup.key` together with the salt
//! and KDF parameters it was derived with, so archives always record the
//! parameters that actually produced the cached key.

use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand::RngCore;

use super::BackupError;

const MAGIC: &[u8; 4] = b"GRNT";
const FORMAT_VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;
const HEADER_LEN: usize = 4 + 1 + 12 + SALT_LEN + NONCE_LEN;

pub(crate) const MIN_PASSPHRASE_LEN: usize = 8;

/// Name of the cached key file inside `.granit/`. Must never be included in
/// the archive itself.
pub(crate) const KEY_FILE_NAME: &str = "backup.key";
const KEY_FILE_HEADER: &str = "GRANIT-BACKUP-KEY v1";

/// Argon2id cost parameters. Fixed for new backups, but recorded in both the
/// key cache and every archive header, so older archives (and caches) made
/// with different parameters keep working.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

const DEFAULT_KDF_PARAMS: KdfParams = KdfParams {
    m_kib: 64 * 1024,
    t: 3,
    p: 4,
};

/// The cached derived key plus everything needed to stamp archive headers.
pub(crate) struct CachedKey {
    pub key: [u8; KEY_LEN],
    pub salt: [u8; SALT_LEN],
    pub params: KdfParams,
}

fn key_file_path(granit_dir: &Path) -> PathBuf {
    granit_dir.join(KEY_FILE_NAME)
}

fn derive_key(
    passphrase: &str,
    salt: &[u8; SALT_LEN],
    params: KdfParams,
) -> Result<[u8; KEY_LEN], BackupError> {
    let argon_params = Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN))
        .map_err(|_| BackupError::Encrypt)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut key = [0u8; KEY_LEN];
    argon
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|_| BackupError::Encrypt)?;
    Ok(key)
}

/// Whether a cached key exists for this cave.
pub(crate) fn has_key_file(granit_dir: &Path) -> bool {
    key_file_path(granit_dir).is_file()
}

/// Derive a key from `passphrase` with a fresh salt and cache it in
/// `.granit/backup.key`. Deliberately slow (Argon2id) — call off the main
/// thread. Replacing an existing key file is fine: older archives carry
/// their own salt and parameters in the header.
pub(crate) fn set_passphrase(granit_dir: &Path, passphrase: &str) -> Result<(), BackupError> {
    if passphrase.chars().count() < MIN_PASSPHRASE_LEN {
        return Err(BackupError::WeakPassphrase);
    }
    let mut salt = [0u8; SALT_LEN];
    rand::rng().fill_bytes(&mut salt);
    let params = DEFAULT_KDF_PARAMS;
    let key = derive_key(passphrase, &salt, params)?;

    let contents = format!(
        "{KEY_FILE_HEADER}\nargon2id m={} t={} p={}\nsalt {}\nkey {}\n",
        params.m_kib,
        params.t,
        params.p,
        BASE64.encode(salt),
        BASE64.encode(key),
    );
    let path = key_file_path(granit_dir);
    crate::cave::write_atomic(&path, contents.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Load the cached key. A missing file means no passphrase has been set; a
/// present-but-unreadable file is an error and is never silently replaced —
/// regenerating the key would orphan every existing backup.
pub(crate) fn load_key(granit_dir: &Path) -> Result<CachedKey, BackupError> {
    let path = key_file_path(granit_dir);
    if !path.is_file() {
        return Err(BackupError::PassphraseNotSet);
    }
    let contents = std::fs::read_to_string(&path)?;
    parse_key_file(&contents)
}

fn parse_key_file(contents: &str) -> Result<CachedKey, BackupError> {
    let invalid = |what: &str| BackupError::InvalidKeyFile(what.to_string());
    let mut lines = contents.lines();
    if lines.next() != Some(KEY_FILE_HEADER) {
        return Err(invalid("unrecognized header"));
    }

    let mut params: Option<KdfParams> = None;
    let mut salt: Option<[u8; SALT_LEN]> = None;
    let mut key: Option<[u8; KEY_LEN]> = None;
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("argon2id ") {
            let mut m = None;
            let mut t = None;
            let mut p = None;
            for part in rest.split_whitespace() {
                let (name, value) = part
                    .split_once('=')
                    .ok_or_else(|| invalid("bad kdf line"))?;
                let value: u32 = value.parse().map_err(|_| invalid("bad kdf value"))?;
                match name {
                    "m" => m = Some(value),
                    "t" => t = Some(value),
                    "p" => p = Some(value),
                    _ => return Err(invalid("unknown kdf parameter")),
                }
            }
            params = Some(KdfParams {
                m_kib: m.ok_or_else(|| invalid("missing m"))?,
                t: t.ok_or_else(|| invalid("missing t"))?,
                p: p.ok_or_else(|| invalid("missing p"))?,
            });
        } else if let Some(rest) = line.strip_prefix("salt ") {
            let bytes = BASE64.decode(rest).map_err(|_| invalid("bad salt"))?;
            salt = Some(bytes.try_into().map_err(|_| invalid("bad salt length"))?);
        } else if let Some(rest) = line.strip_prefix("key ") {
            let bytes = BASE64.decode(rest).map_err(|_| invalid("bad key"))?;
            key = Some(bytes.try_into().map_err(|_| invalid("bad key length"))?);
        } else {
            return Err(invalid("unrecognized line"));
        }
    }
    Ok(CachedKey {
        key: key.ok_or_else(|| invalid("missing key"))?,
        salt: salt.ok_or_else(|| invalid("missing salt"))?,
        params: params.ok_or_else(|| invalid("missing kdf parameters"))?,
    })
}

fn build_header(cached: &CachedKey, nonce: &[u8; NONCE_LEN]) -> Vec<u8> {
    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.push(FORMAT_VERSION);
    header.extend_from_slice(&cached.params.m_kib.to_le_bytes());
    header.extend_from_slice(&cached.params.t.to_le_bytes());
    header.extend_from_slice(&cached.params.p.to_le_bytes());
    header.extend_from_slice(&cached.salt);
    header.extend_from_slice(nonce);
    header
}

/// Encrypt `plaintext` into a self-describing container (header + ciphertext).
pub(crate) fn encrypt(cached: &CachedKey, plaintext: &[u8]) -> Result<Vec<u8>, BackupError> {
    let mut nonce = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce);
    let header = build_header(cached, &nonce);

    let cipher = XChaCha20Poly1305::new((&cached.key).into());
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: &header,
            },
        )
        .map_err(|_| BackupError::Encrypt)?;

    let mut container = header;
    container.extend_from_slice(&ciphertext);
    Ok(container)
}

/// Decrypt a container using only the passphrase — the salt and KDF
/// parameters come from the header, so this works on a machine that has
/// never seen the cave. Used by tests today, restore later.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn decrypt(passphrase: &str, container: &[u8]) -> Result<Vec<u8>, BackupError> {
    if container.len() < HEADER_LEN || &container[..4] != MAGIC {
        return Err(BackupError::Decrypt);
    }
    if container[4] != FORMAT_VERSION {
        return Err(BackupError::Decrypt);
    }
    let read_u32 = |offset: usize| {
        u32::from_le_bytes(container[offset..offset + 4].try_into().expect("in bounds"))
    };
    let params = KdfParams {
        m_kib: read_u32(5),
        t: read_u32(9),
        p: read_u32(13),
    };
    let salt: [u8; SALT_LEN] = container[17..17 + SALT_LEN].try_into().expect("in bounds");
    let nonce = &container[17 + SALT_LEN..HEADER_LEN];
    let header = &container[..HEADER_LEN];
    let ciphertext = &container[HEADER_LEN..];

    let key = derive_key(passphrase, &salt, params)?;
    let cipher = XChaCha20Poly1305::new((&key).into());
    cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad: header,
            },
        )
        .map_err(|_| BackupError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cheap parameters so tests don't burn 64 MiB × 3 iterations per derive.
    fn fast_key(passphrase: &str) -> CachedKey {
        let params = KdfParams {
            m_kib: 8,
            t: 1,
            p: 1,
        };
        let salt = [7u8; SALT_LEN];
        CachedKey {
            key: derive_key(passphrase, &salt, params).unwrap(),
            salt,
            params,
        }
    }

    #[test]
    fn encrypt_decrypt_round_trip_needs_only_the_passphrase() {
        let cached = fast_key("correct horse battery");
        let container = encrypt(&cached, b"cave contents").unwrap();
        assert_eq!(&container[..4], MAGIC);
        let plain = decrypt("correct horse battery", &container).unwrap();
        assert_eq!(plain, b"cave contents");
    }

    #[test]
    fn wrong_passphrase_is_rejected() {
        let cached = fast_key("correct horse battery");
        let container = encrypt(&cached, b"cave contents").unwrap();
        assert!(matches!(
            decrypt("wrong passphrase!", &container),
            Err(BackupError::Decrypt)
        ));
    }

    #[test]
    fn tampered_container_is_rejected() {
        let cached = fast_key("correct horse battery");
        let mut container = encrypt(&cached, b"cave contents").unwrap();
        // Flip a bit in the ciphertext…
        let last = container.len() - 1;
        container[last] ^= 1;
        assert!(matches!(
            decrypt("correct horse battery", &container),
            Err(BackupError::Decrypt)
        ));
        // …and in the header (covered as AEAD associated data).
        let mut container = encrypt(&cached, b"cave contents").unwrap();
        container[5] ^= 1;
        assert!(matches!(
            decrypt("correct horse battery", &container),
            Err(BackupError::Decrypt)
        ));
    }

    #[test]
    fn set_passphrase_writes_reloadable_key_file() {
        let dir = tempfile::tempdir().unwrap();
        set_passphrase(dir.path(), "a long enough passphrase").unwrap();
        assert!(has_key_file(dir.path()));

        let cached = load_key(dir.path()).unwrap();
        assert_eq!(cached.params, DEFAULT_KDF_PARAMS);
        // The cached key must match a fresh derivation from the stored salt.
        let rederived =
            derive_key("a long enough passphrase", &cached.salt, cached.params).unwrap();
        assert_eq!(cached.key, rederived);
    }

    #[test]
    fn short_passphrases_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            set_passphrase(dir.path(), "short"),
            Err(BackupError::WeakPassphrase)
        ));
        assert!(!has_key_file(dir.path()));
    }

    #[test]
    fn missing_key_file_reports_passphrase_not_set() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            load_key(dir.path()),
            Err(BackupError::PassphraseNotSet)
        ));
    }

    #[test]
    fn corrupt_key_file_errors_and_is_never_regenerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(KEY_FILE_NAME);
        std::fs::write(&path, "not a key file").unwrap();
        assert!(matches!(
            load_key(dir.path()),
            Err(BackupError::InvalidKeyFile(_))
        ));
        // The corrupt file must still be there, untouched.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not a key file");
    }

    #[test]
    fn key_file_round_trips_through_parser() {
        let cached = fast_key("correct horse battery");
        let contents = format!(
            "{KEY_FILE_HEADER}\nargon2id m={} t={} p={}\nsalt {}\nkey {}\n",
            cached.params.m_kib,
            cached.params.t,
            cached.params.p,
            BASE64.encode(cached.salt),
            BASE64.encode(cached.key),
        );
        let parsed = parse_key_file(&contents).unwrap();
        assert_eq!(parsed.key, cached.key);
        assert_eq!(parsed.salt, cached.salt);
        assert_eq!(parsed.params, cached.params);
    }
}
