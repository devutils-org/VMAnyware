//! Optional Linux Secret Service unlock secret, bound to one app-owned key.
use super::{password_bounds, public, IdentityStore, PrivateKey, PublicIdentity, Zeroizing};
use dbus_secret_service::{EncryptionType, SecretService};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, os::unix::ffi::OsStrExt};
const ERROR: &str =
    "System key store unavailable, locked or ambiguous. Unlock it in your desktop and retry.";
const MISSING: &str = "No saved unlock secret for this application SSH identity.";
struct Scope {
    directory: String,
    fingerprint: String,
}
impl Scope {
    fn attributes(&self) -> HashMap<&str, &str> {
        HashMap::from([
            ("application", "dev.vmanyware.client"),
            ("purpose", "ssh-identity-unlock-v1"),
            ("scope", &self.directory),
            ("identity", &self.fingerprint),
        ])
    }
}
fn service() -> Result<SecretService, String> {
    SecretService::connect_with_max_prompt_timeout(EncryptionType::Dh, 0).map_err(|_| ERROR.into())
}
impl IdentityStore {
    fn vault_scope(&self, encrypted: &PrivateKey) -> Result<Scope, String> {
        let path = self.directory.canonicalize().map_err(|_| ERROR)?;
        Ok(Scope {
            directory: format!("{:x}", Sha256::digest(path.as_os_str().as_bytes())),
            fingerprint: public(encrypted)?.fingerprint,
        })
    }
    pub fn remember(&self, password: Zeroizing<String>) -> Result<PublicIdentity, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        password_bounds(&password)?;
        let encrypted = self.read_encrypted()?;
        let key = encrypted
            .decrypt(password.as_bytes())
            .map_err(|_| super::UNLOCK)?;
        let scope = self.vault_scope(&encrypted)?;
        let service = service()?;
        let collection = service.get_default_collection().map_err(|_| ERROR)?;
        if collection.is_locked().map_err(|_| ERROR)? {
            return Err(ERROR.into());
        }
        let items = collection
            .search_items(scope.attributes())
            .map_err(|_| ERROR)?;
        match items.as_slice() {
            [] => {
                collection
                    .create_item(
                        "VMAnyware SSH identity unlock",
                        scope.attributes(),
                        password.as_bytes(),
                        false,
                        "text/plain",
                    )
                    .map_err(|_| ERROR)?;
            }
            [item] if !item.is_locked().map_err(|_| ERROR)? => {
                item.set_secret(password.as_bytes(), "text/plain")
                    .map_err(|_| ERROR)?;
            }
            _ => return Err(ERROR.into()),
        }
        // Confirm the one scoped item, including after a racing create. Never
        // turn an ambiguous write into a successful save acknowledgement.
        let retained = collection
            .search_items(scope.attributes())
            .map_err(|_| ERROR)?;
        let [item] = retained.as_slice() else {
            return Err(ERROR.into());
        };
        if item.is_locked().map_err(|_| ERROR)? {
            return Err(ERROR.into());
        }
        let saved = Zeroizing::new(item.get_secret().map_err(|_| ERROR)?);
        if saved.as_slice() != password.as_bytes() {
            return Err(ERROR.into());
        }
        public(&key)
    }
    pub fn forget(&self, confirmed: bool) -> Result<(), String> {
        if !confirmed {
            return Err("Confirm forgetting this identity's saved unlock secret.".into());
        }
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        let scope = self.vault_scope(&self.read_encrypted()?)?;
        let service = service()?;
        let collection = service.get_default_collection().map_err(|_| ERROR)?;
        if collection.is_locked().map_err(|_| ERROR)? {
            return Err(ERROR.into());
        }
        let items = collection
            .search_items(scope.attributes())
            .map_err(|_| ERROR)?;
        match items.as_slice() {
            [] => Ok(()),
            [item] if !item.is_locked().map_err(|_| ERROR)? => {
                item.delete().map_err(|_| ERROR.into())
            }
            _ => Err(ERROR.into()),
        }
    }
    pub fn unlock_system(&self) -> Result<PublicIdentity, String> {
        public(&self.signing_system()?)
    }
    pub(in crate::ssh) fn signing_system(&self) -> Result<PrivateKey, String> {
        let _guard = self.busy.try_lock().map_err(|_| "SSH identity is busy")?;
        let encrypted = self.read_encrypted()?;
        let scope = self.vault_scope(&encrypted)?;
        let service = service()?;
        let collection = service.get_default_collection().map_err(|_| ERROR)?;
        if collection.is_locked().map_err(|_| ERROR)? {
            return Err(ERROR.into());
        }
        let items = collection
            .search_items(scope.attributes())
            .map_err(|_| ERROR)?;
        let item = match items.as_slice() {
            [] => return Err(MISSING.into()),
            [item] => item,
            _ => return Err(ERROR.into()),
        };
        if item.is_locked().map_err(|_| ERROR)? {
            return Err(ERROR.into());
        }
        let secret = Zeroizing::new(item.get_secret().map_err(|_| ERROR)?);
        if secret.len() > 1024 {
            return Err(ERROR.into());
        }
        let password = std::str::from_utf8(&secret).map_err(|_| ERROR)?;
        password_bounds(password).map_err(|_| ERROR)?;
        encrypted
            .decrypt(password)
            .map_err(|_| "Saved unlock secret does not match this application SSH identity.".into())
    }
}

#[cfg(test)]
pub(in crate::ssh) mod tests {
    use super::*;
    fn password() -> Zeroizing<String> {
        Zeroizing::new("owned vault phrase only 42".into())
    }
    #[test]
    fn vault_namespace_binds_directory_and_public_identity_without_secret_attributes() {
        let root = tempfile::tempdir().unwrap();
        let first = IdentityStore::new(root.path().join("first"));
        first.create(password()).unwrap();
        let second = IdentityStore::new(root.path().join("second"));
        second.create(password()).unwrap();
        let a = first.vault_scope(&first.read_encrypted().unwrap()).unwrap();
        let b = second
            .vault_scope(&second.read_encrypted().unwrap())
            .unwrap();
        assert_ne!(a.directory, b.directory);
        assert_ne!(a.fingerprint, b.fingerprint);
        assert_eq!(a.directory.len(), 64);
        assert_eq!(a.attributes().len(), 4);
        assert!(!format!("{:?}", a.attributes()).contains("owned vault phrase"));
        assert!(first.forget(false).is_err()); // Refuses before any bus connection.
    }
    pub(in crate::ssh) fn require_owned_bus() -> std::path::PathBuf {
        let root = std::path::PathBuf::from(
            std::env::var("VMANYWARE_OWNED_VAULT_TEST").expect("Use the owned vault runner"),
        );
        assert!(
            root.is_absolute()
                && root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("vmanyware-vault-")
        );
        assert!(std::env::var("DBUS_SESSION_BUS_ADDRESS")
            .unwrap()
            .starts_with("unix:path="));
        assert!(root.join("owned-test-bus").is_file());
        root
    }
    #[test]
    #[ignore = "Requires an explicitly isolated owned Secret Service; use the vault runner"]
    fn owned_vault_roundtrip_refuses_other_identity_duplicates_wrong_secret_and_locked_store() {
        let root = require_owned_bus();
        let first = IdentityStore::new(root.join("identity"));
        let original = first.create(password()).unwrap();
        let before = std::fs::read(first.directory.join("identity.openssh")).unwrap();
        assert_eq!(first.unlock_system().err().unwrap(), MISSING);
        assert!(first
            .remember(Zeroizing::new("wrong owned vault phrase".into()))
            .is_err());
        first.remember(password()).unwrap();
        first.remember(password()).unwrap();
        let reopened = IdentityStore::new(first.directory.clone());
        assert_eq!(
            reopened.unlock_system().unwrap().fingerprint,
            original.fingerprint
        );
        let second = IdentityStore::new(root.join("other"));
        second.create(password()).unwrap();
        assert_eq!(second.unlock_system().err().unwrap(), MISSING);
        let scope = first.vault_scope(&first.read_encrypted().unwrap()).unwrap();
        let service = service().unwrap();
        let collection = service.get_default_collection().unwrap();
        let items = collection.search_items(scope.attributes()).unwrap();
        assert_eq!(items.len(), 1);
        items[0]
            .set_secret(b"incorrect saved phrase only 42", "text/plain")
            .unwrap();
        assert!(first
            .unlock_system()
            .err()
            .unwrap()
            .starts_with("Saved unlock secret does not match"));
        first.remember(password()).unwrap();
        let duplicate = collection
            .create_item(
                "Owned duplicate",
                scope.attributes(),
                password().as_bytes(),
                false,
                "text/plain",
            )
            .unwrap();
        assert_eq!(first.unlock_system().err().unwrap(), ERROR);
        assert_eq!(first.forget(true).err().unwrap(), ERROR);
        duplicate.delete().unwrap();
        first.forget(true).unwrap();
        first.forget(true).unwrap();
        assert_eq!(first.unlock_system().err().unwrap(), MISSING);
        first.remember(password()).unwrap();
        collection.lock().unwrap();
        assert_eq!(first.unlock_system().err().unwrap(), ERROR);
        assert_eq!(first.remember(password()).err().unwrap(), ERROR);
        assert_eq!(first.forget(true).err().unwrap(), ERROR);
        assert_eq!(
            std::fs::read(first.directory.join("identity.openssh")).unwrap(),
            before
        );
        assert_eq!(
            first.unlock(password()).unwrap().fingerprint,
            original.fingerprint
        );
    }
}
