//! Android's OS-protected unlock secret; private SSH key stays encrypted in app storage.
#[cfg(target_os = "android")]
use super::{password_bounds, PublicIdentity, Zeroizing};
use super::{public, IdentityStore, PrivateKey};
use sha2::{Digest, Sha256};
use std::os::unix::ffi::OsStrExt;
impl IdentityStore {
    fn android_scope(&self, encrypted: &PrivateKey) -> Result<String, String> {
        let path = self
            .directory
            .canonicalize()
            .map_err(|_| super::UNAVAILABLE)?;
        let mut digest = Sha256::new();
        digest.update(b"vmanyware-android-ssh-unlock-v1\0");
        digest.update(path.as_os_str().as_bytes());
        digest.update([0]);
        digest.update(public(encrypted)?.fingerprint.as_bytes());
        Ok(format!("{:x}", digest.finalize()))
    }
    #[cfg(target_os = "android")]
    pub fn remember(&self, password: Zeroizing<String>) -> Result<PublicIdentity, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        password_bounds(&password)?;
        let encrypted = self.read_encrypted()?;
        let key = encrypted
            .decrypt(password.as_bytes())
            .map_err(|_| super::UNLOCK)?;
        let scope = self.android_scope(&encrypted)?;
        crate::android_identity::save(&scope, password.as_bytes())?;
        let saved = crate::android_identity::read(&scope)?;
        if saved.as_slice() != password.as_bytes() {
            return Err("Android key store did not confirm this saved unlock secret.".into());
        }
        public(&key)
    }
    #[cfg(target_os = "android")]
    pub fn forget(&self, confirmed: bool) -> Result<(), String> {
        if !confirmed {
            return Err("Confirm forgetting this identity's saved unlock secret.".into());
        }
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        let scope = self.android_scope(&self.read_encrypted()?)?;
        crate::android_identity::forget(&scope)
    }
    #[cfg(target_os = "android")]
    pub fn unlock_system(&self) -> Result<PublicIdentity, String> {
        public(&self.signing_system()?)
    }
    #[cfg(target_os = "android")]
    pub(in crate::ssh) fn signing_system(&self) -> Result<PrivateKey, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        let encrypted = self.read_encrypted()?;
        let scope = self.android_scope(&encrypted)?;
        let secret = crate::android_identity::read(&scope)?;
        let password = std::str::from_utf8(&secret).map_err(|_| super::UNLOCK)?;
        password_bounds(password)?;
        encrypted
            .decrypt(password)
            .map_err(|_| "Saved unlock secret does not match this application SSH identity.".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn android_unlock_scope_is_stable_and_binds_directory_and_identity() {
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
        let a = first.read_encrypted().unwrap();
        let b = second.read_encrypted().unwrap();
        let scope = first.android_scope(&a).unwrap();
        assert_eq!(scope.len(), 64);
        assert!(scope
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        assert_eq!(first.android_scope(&a).unwrap(), scope);
        assert_ne!(first.android_scope(&b).unwrap(), scope);
        assert_ne!(second.android_scope(&a).unwrap(), scope);
    }
}
