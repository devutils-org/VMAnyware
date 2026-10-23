//! Explicit macOS Keychain unlock-secret actions; SSH private keys stay in app storage.
#[cfg(target_os = "macos")]
use super::{password_bounds, PublicIdentity, Zeroizing};
use super::{public, IdentityStore, PrivateKey};
#[cfg(target_os = "macos")]
use security_framework::os::macos::keychain::SecKeychain;
use sha2::{Digest, Sha256};
use std::os::unix::ffi::OsStrExt;
#[cfg(target_os = "macos")]
const SERVICE: &str = "dev.vmanyware.client.ssh-unlock.v1";
#[cfg(target_os = "macos")]
const UNAVAILABLE: &str = "macOS Keychain unavailable or access was not granted. Unlock the keychain or use the passphrase.";
#[cfg(target_os = "macos")]
const MISSING: &str = "No saved unlock secret for this application SSH identity.";
// Apple's errSecItemNotFound. Other errors must never be treated as missing.
#[cfg(target_os = "macos")]
const ITEM_NOT_FOUND: i32 = -25300;

impl IdentityStore {
    fn macos_scope(&self, encrypted: &PrivateKey) -> Result<String, String> {
        let path = self
            .directory
            .canonicalize()
            .map_err(|_| super::UNAVAILABLE)?;
        let mut digest = Sha256::new();
        digest.update(b"vmanyware-macos-ssh-unlock-v1\0");
        digest.update(path.as_os_str().as_bytes());
        digest.update([0]);
        digest.update(public(encrypted)?.fingerprint.as_bytes());
        Ok(format!("{:x}", digest.finalize()))
    }
    #[cfg(target_os = "macos")]
    pub fn remember(&self, password: Zeroizing<String>) -> Result<PublicIdentity, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        password_bounds(&password)?;
        let encrypted = self.read_encrypted()?;
        let key = encrypted
            .decrypt(password.as_bytes())
            .map_err(|_| super::UNLOCK)?;
        let scope = self.macos_scope(&encrypted)?;
        // Use one explicit default keychain, not an unrestricted search list.
        // Do not use set_generic_password: its fallback adds after any find error.
        let keychain = SecKeychain::default().map_err(|_| UNAVAILABLE)?;
        match keychain.find_generic_password(SERVICE, &scope) {
            Ok((_saved, mut item)) => item
                .set_password(password.as_bytes())
                .map_err(|_| UNAVAILABLE)?,
            Err(error) if error.code() == ITEM_NOT_FOUND => keychain
                .add_generic_password(SERVICE, &scope, password.as_bytes())
                .map_err(|_| UNAVAILABLE)?,
            Err(_) => return Err(UNAVAILABLE.into()),
        }
        let saved = keychain_secret(&keychain, &scope)?;
        if saved.as_slice() != password.as_bytes() {
            return Err("macOS Keychain did not confirm this saved unlock secret.".into());
        }
        public(&key)
    }
    #[cfg(target_os = "macos")]
    pub fn forget(&self, confirmed: bool) -> Result<(), String> {
        if !confirmed {
            return Err("Confirm forgetting this identity's saved unlock secret.".into());
        }
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        let scope = self.macos_scope(&self.read_encrypted()?)?;
        let keychain = SecKeychain::default().map_err(|_| UNAVAILABLE)?;
        match keychain.find_generic_password(SERVICE, &scope) {
            Ok((_saved, item)) => item.delete(),
            Err(error) if error.code() == ITEM_NOT_FOUND => return Ok(()),
            Err(_) => return Err(UNAVAILABLE.into()),
        }
        // The wrapper's legacy delete returns no status. Require absence explicitly.
        match keychain.find_generic_password(SERVICE, &scope) {
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
            _ => Err("macOS Keychain did not confirm forgetting this saved unlock secret.".into()),
        }
    }
    #[cfg(target_os = "macos")]
    pub fn unlock_system(&self) -> Result<PublicIdentity, String> {
        public(&self.signing_system()?)
    }
    #[cfg(target_os = "macos")]
    pub(in crate::ssh) fn signing_system(&self) -> Result<PrivateKey, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        let encrypted = self.read_encrypted()?;
        let scope = self.macos_scope(&encrypted)?;
        let keychain = SecKeychain::default().map_err(|_| UNAVAILABLE)?;
        let secret = keychain_secret(&keychain, &scope)?;
        let password = std::str::from_utf8(&secret).map_err(|_| super::UNLOCK)?;
        password_bounds(password)?;
        encrypted
            .decrypt(password)
            .map_err(|_| "Saved unlock secret does not match this application SSH identity.".into())
    }
}
#[cfg(target_os = "macos")]
fn keychain_secret(keychain: &SecKeychain, scope: &str) -> Result<Zeroizing<Vec<u8>>, String> {
    let (data, _item) = keychain
        .find_generic_password(SERVICE, scope)
        .map_err(|error| {
            if error.code() == ITEM_NOT_FOUND {
                MISSING
            } else {
                UNAVAILABLE
            }
        })?;
    if data.is_empty() || data.len() > 1024 {
        return Err(UNAVAILABLE.into());
    }
    Ok(Zeroizing::new(data.to_vec()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn macos_keychain_scope_binds_directory_and_identity_without_keychain_access() {
        let root = tempfile::tempdir().unwrap();
        let first = IdentityStore::new(root.path().join("first"));
        let second = IdentityStore::new(root.path().join("second"));
        first
            .create(super::super::Zeroizing::new(
                "owned scope phrase only 42".into(),
            ))
            .unwrap();
        second
            .create(super::super::Zeroizing::new(
                "owned scope phrase only 42".into(),
            ))
            .unwrap();
        let encrypted = first.read_encrypted().unwrap();
        let scope = first.macos_scope(&encrypted).unwrap();
        assert_eq!(scope.len(), 64);
        assert!(scope
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        assert_eq!(first.macos_scope(&encrypted).unwrap(), scope);
        assert_ne!(second.macos_scope(&encrypted).unwrap(), scope);
        assert_ne!(
            first
                .macos_scope(&second.read_encrypted().unwrap())
                .unwrap(),
            scope
        );
    }
}
