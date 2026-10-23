//! Fixed app-owned encrypted Ed25519 identity. Never imports user key files.
#[cfg(any(target_os = "android", all(test, target_os = "linux")))]
#[path = "identity_android.rs"]
mod android;
#[cfg(any(target_os = "macos", all(test, target_os = "linux")))]
#[path = "identity_macos.rs"]
mod macos;
#[cfg(target_os = "linux")]
#[path = "identity_vault.rs"]
pub(super) mod vault;
use ssh_key::{Algorithm, Cipher, HashAlg, Kdf, LineEnding, PrivateKey};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use zeroize::Zeroizing;

const UNAVAILABLE: &str = "Application SSH identity unavailable";
const UNLOCK: &str = "Cannot unlock application SSH identity";
const MAX_BYTES: u64 = 4096;
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicIdentity {
    public_key: String,
    fingerprint: String,
}
#[derive(Clone)]
pub struct IdentityStore {
    directory: PathBuf,
    // Serialize expensive encryption/decryption; never queue unbounded KDF work.
    busy: Arc<Mutex<()>>,
}
impl IdentityStore {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            busy: Arc::new(Mutex::new(())),
        }
    }
    fn directory(&self, create: bool) -> Result<(), String> {
        if create {
            let parent = self.directory.parent().ok_or(UNAVAILABLE)?;
            fs::create_dir_all(parent).map_err(|_| UNAVAILABLE)?;
            match fs::DirBuilder::new().mode(0o700).create(&self.directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(UNAVAILABLE.into()),
            }
        }
        private_metadata(&self.directory, true)
    }
    pub fn create(&self, password: Zeroizing<String>) -> Result<PublicIdentity, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        password_bounds(&password)?;
        self.directory(true)?;
        let path = self.directory.join("identity.openssh");
        if fs::symlink_metadata(&path).is_ok() {
            return Err("An application SSH identity already exists; unlock it instead".into());
        }
        let key =
            PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).map_err(|_| UNAVAILABLE)?;
        let encrypted = key
            .encrypt_with_cipher(&mut rand::rng(), Cipher::Aes256Gcm, password.as_bytes())
            .map_err(|_| UNAVAILABLE)?;
        let pem = encrypted
            .to_openssh(LineEnding::LF)
            .map_err(|_| UNAVAILABLE)?;
        // Atomic no-clobber publication: a failed write cannot expose a partial key.
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| UNAVAILABLE)?;
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| UNAVAILABLE)?;
        temporary
            .write_all(pem.as_bytes())
            .map_err(|_| UNAVAILABLE)?;
        temporary.as_file().sync_all().map_err(|_| UNAVAILABLE)?;
        temporary
            .persist_noclobber(path)
            .map_err(|_| "Cannot save identity; it may already exist")?;
        File::open(&self.directory)
            .and_then(|file| file.sync_all())
            .map_err(|_| UNAVAILABLE)?;
        public(&key)
    }
    pub fn unlock(&self, password: Zeroizing<String>) -> Result<PublicIdentity, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        password_bounds(&password)?;
        let key = self.read_key(&password)?;
        public(&key)
    }
    pub(super) fn signing_key(&self, password: Zeroizing<String>) -> Result<PrivateKey, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        password_bounds(&password)?;
        self.read_key(&password)
    }
    fn read_key(&self, password: &str) -> Result<PrivateKey, String> {
        self.read_encrypted()?
            .decrypt(password)
            .map_err(|_| UNLOCK.into())
    }
    fn read_encrypted(&self) -> Result<PrivateKey, String> {
        self.directory(false).map_err(|_| UNLOCK)?;
        let path = self.directory.join("identity.openssh");
        private_metadata(&path, false).map_err(|_| UNLOCK)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|_| UNLOCK)?;
        let metadata = file.metadata().map_err(|_| UNLOCK)?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.len() > MAX_BYTES
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(UNLOCK.into());
        }
        let mut encoded = Zeroizing::new(Vec::new());
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut encoded)
            .map_err(|_| UNLOCK)?;
        if encoded.len() as u64 > MAX_BYTES {
            return Err(UNLOCK.into());
        }
        let encrypted = PrivateKey::from_openssh(&*encoded).map_err(|_| UNLOCK)?;
        // Only accept our fixed format and bounded KDF cost, including on corrupted files.
        if encrypted.algorithm() != Algorithm::Ed25519
            || encrypted.cipher() != Cipher::Aes256Gcm
            || !matches!(encrypted.kdf(), Kdf::Bcrypt { salt, rounds: 16 } if salt.len() == 16)
        {
            return Err(UNLOCK.into());
        }
        Ok(encrypted)
    }
}
fn password_bounds(password: &str) -> Result<(), String> {
    if password.chars().count() < 16
        || password.len() > 1024
        || password.trim().is_empty()
        || password.contains('\0')
    {
        Err("Use a secret passphrase of 16 or more characters (at most 1024 bytes)".into())
    } else {
        Ok(())
    }
}
fn public(key: &PrivateKey) -> Result<PublicIdentity, String> {
    Ok(PublicIdentity {
        public_key: key.public_key().to_openssh().map_err(|_| UNAVAILABLE)?,
        fingerprint: key.public_key().fingerprint(HashAlg::Sha256).to_string(),
    })
}
fn private_metadata(path: &Path, directory: bool) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| UNAVAILABLE)?;
    // SAFETY: geteuid has no preconditions and does not access credentials.
    let uid = unsafe { libc::geteuid() };
    if metadata.file_type().is_symlink()
        || metadata.uid() != uid
        || metadata.permissions().mode() & 0o077 != 0
        || (if directory {
            !metadata.is_dir()
        } else {
            !metadata.is_file()
        })
    {
        return Err(UNAVAILABLE.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn password() -> Zeroizing<String> {
        Zeroizing::new("fixture passphrase only 42".into())
    }
    #[test]
    fn encrypted_identity_reopens_without_exposing_or_replacing_private_key() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("identity");
        let store = IdentityStore::new(path.clone());
        let identity = store.create(password()).unwrap();
        let ciphertext = fs::read(path.join("identity.openssh")).unwrap();
        let encrypted = PrivateKey::from_openssh(&ciphertext).unwrap();
        assert!(encrypted.is_encrypted());
        assert_eq!(encrypted.cipher(), Cipher::Aes256Gcm);
        assert!(identity.public_key.starts_with("ssh-ed25519 "));
        let serialized = serde_json::to_string(&identity).unwrap();
        assert!(!serialized.contains("PRIVATE"));
        assert!(!serialized.contains("passphrase"));
        drop(store);
        let reopened = IdentityStore::new(path.clone());
        assert_eq!(
            reopened.unlock(password()).unwrap().public_key,
            identity.public_key
        );
        assert_eq!(
            reopened.unlock(password()).unwrap().fingerprint,
            identity.fingerprint
        );
        assert_eq!(
            reopened
                .unlock(Zeroizing::new("different passphrase 42".into()))
                .err()
                .unwrap(),
            UNLOCK
        );
        assert!(reopened.create(password()).is_err());
        assert_eq!(fs::read(path.join("identity.openssh")).unwrap(), ciphertext);
    }
    #[test]
    fn encrypted_identity_rejects_bad_storage_passwords_and_ciphertext() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("identity");
        let store = IdentityStore::new(path.clone());
        assert!(store.create(Zeroizing::new("short".into())).is_err());
        assert!(!path.exists());
        store.create(password()).unwrap();
        let key_path = path.join("identity.openssh");
        let ciphertext = fs::read(&key_path).unwrap();
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(store.unlock(password()).is_err());
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();
        let guard = store.busy.lock().unwrap();
        assert!(matches!(store.unlock(password()), Err(error) if error == "SSH identity is busy"));
        drop(guard);
        fs::write(&key_path, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        assert!(store.unlock(password()).is_err());
        let encrypted = PrivateKey::from_openssh(&ciphertext).unwrap();
        let mut encoded = encrypted.to_bytes().unwrap().to_vec();
        // The last byte belongs to the GCM tag: preserve a valid envelope but
        // reject authenticated ciphertext tampering.
        *encoded.last_mut().unwrap() ^= 1;
        let tampered = PrivateKey::from_bytes(&encoded).unwrap();
        fs::write(
            &key_path,
            tampered.to_openssh(LineEnding::LF).unwrap().as_bytes(),
        )
        .unwrap();
        assert!(store.unlock(password()).is_err());
        let plain = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap();
        fs::write(
            &key_path,
            plain.to_openssh(LineEnding::LF).unwrap().as_bytes(),
        )
        .unwrap();
        assert!(store.unlock(password()).is_err());
        fs::write(&key_path, b"corrupted").unwrap();
        assert!(store.unlock(password()).is_err());
        fs::remove_file(&key_path).unwrap();
        let other = root.path().join("other");
        fs::write(&other, ciphertext).unwrap();
        std::os::unix::fs::symlink(&other, &key_path).unwrap();
        assert!(store.unlock(password()).is_err());
    }
}
