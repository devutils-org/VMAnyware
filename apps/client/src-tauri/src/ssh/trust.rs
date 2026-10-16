//! Managed public host trust. Never reads SSH configuration or private keys.
use russh::keys::{HashAlg, PublicKey};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::IpAddr,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 65536;
const MAX_ENTRIES: usize = 128;
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidEndpoint,
    InvalidStore,
    Io,
    Busy,
    ChangedKey,
    ConfirmationMismatch,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidEndpoint => "Invalid SSH endpoint",
            Self::InvalidStore => "Invalid host trust store",
            Self::Io => "Host trust storage unavailable",
            Self::Busy => "Host trust storage busy",
            Self::ChangedKey => "SSH host key changed",
            Self::ConfirmationMismatch => "Host fingerprint confirmation did not match",
        })
    }
}
impl std::error::Error for Error {}
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    host: String,
    port: u16,
}
impl Endpoint {
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn new(host: &str, port: u16) -> Result<Self, Error> {
        if port == 0 || host.len() > 253 || host.is_empty() {
            return Err(Error::InvalidEndpoint);
        }
        let host = if let Ok(ip) = host.parse::<IpAddr>() {
            ip.to_string()
        } else {
            if !host.is_ascii()
                || !host.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && label.as_bytes()[0].is_ascii_alphanumeric()
                        && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                        && label
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                })
            {
                return Err(Error::InvalidEndpoint);
            }
            host.to_ascii_lowercase()
        };
        Ok(Self { host, port })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    endpoint: Endpoint,
    public_key: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u8,
    entries: Vec<Record>,
}
/// Opaque observation, only produced after an unknown-key inspection.
pub struct Confirmation {
    endpoint: Endpoint,
    public_key: String,
    fingerprint: String,
}
impl Confirmation {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}
pub enum Inspection {
    Trusted,
    Unknown(Confirmation),
}
pub struct Store {
    directory: PathBuf,
}
impl Store {
    /// Explicitly creates a new private directory; refuses every existing path.
    pub fn create(directory: &Path) -> Result<Self, Error> {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|_| Error::Io)?;
        let path = directory.join("trust.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|_| Error::Io)?;
        file.write_all(br#"{"version":1,"entries":[]}"#)
            .map_err(|_| Error::Io)?;
        Self::open(directory)
    }
    /// Opens an existing initialized private managed directory; missing content fails closed.
    pub fn open(directory: &Path) -> Result<Self, Error> {
        check_private(directory, true)?;
        let store = Self {
            directory: directory.into(),
        };
        let _lock = store.lock()?;
        store.read()?;
        Ok(store)
    }
    fn lock(&self) -> Result<File, Error> {
        check_private(&self.directory, true)?;
        let path = self.directory.join("trust.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)
            .map_err(|_| Error::Io)?;
        check_private(&path, false)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
        loop {
            match file.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(5))
                }
                Err(std::fs::TryLockError::WouldBlock) => return Err(Error::Busy),
                Err(_) => return Err(Error::Io),
            }
        }
        Ok(file)
    }
    fn read(&self) -> Result<Document, Error> {
        let path = self.directory.join("trust.json");
        check_private(&path, false)?;
        let mut bytes = Vec::new();
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| Error::Io)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Io)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(Error::InvalidStore);
        }
        let document: Document = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidStore)?;
        if document.version != 1 || document.entries.len() > MAX_ENTRIES {
            return Err(Error::InvalidStore);
        }
        let mut endpoints = HashSet::new();
        for record in &document.entries {
            if Endpoint::new(&record.endpoint.host, record.endpoint.port)
                .map_err(|_| Error::InvalidStore)?
                != record.endpoint
                || !endpoints.insert(&record.endpoint)
                || canonical_key(&record.public_key).map_err(|_| Error::InvalidStore)?
                    != record.public_key
            {
                return Err(Error::InvalidStore);
            }
        }
        Ok(document)
    }
    pub fn inspect(&self, endpoint: Endpoint, key: &PublicKey) -> Result<Inspection, Error> {
        // Deserialize can construct this type without passing through new().
        if Endpoint::new(&endpoint.host, endpoint.port)? != endpoint {
            return Err(Error::InvalidEndpoint);
        }
        let _lock = self.lock()?;
        let public_key = key.to_openssh().map_err(|_| Error::InvalidStore)?;
        // Strip comments through key-data conversion, not user-controlled text.
        let public_key = canonical_key(&public_key)?;
        match self.read()?.entries.iter().find(|r| r.endpoint == endpoint) {
            Some(record) if record.public_key == public_key => Ok(Inspection::Trusted),
            Some(_) => Err(Error::ChangedKey),
            None => Ok(Inspection::Unknown(Confirmation {
                endpoint,
                public_key,
                fingerprint: key.fingerprint(HashAlg::Sha256).to_string(),
            })),
        }
    }
    pub fn confirm(&self, observed: Confirmation, accepted_fingerprint: &str) -> Result<(), Error> {
        if observed.fingerprint != accepted_fingerprint {
            return Err(Error::ConfirmationMismatch);
        }
        let _lock = self.lock()?;
        let mut document = self.read()?;
        if let Some(record) = document
            .entries
            .iter()
            .find(|r| r.endpoint == observed.endpoint)
        {
            return if record.public_key == observed.public_key {
                Ok(())
            } else {
                Err(Error::ChangedKey)
            };
        }
        if document.entries.len() >= MAX_ENTRIES {
            return Err(Error::InvalidStore);
        }
        document.entries.push(Record {
            endpoint: observed.endpoint,
            public_key: observed.public_key,
        });
        let bytes = serde_json::to_vec(&document).map_err(|_| Error::InvalidStore)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(Error::InvalidStore);
        }
        let mut file = tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| Error::Io)?;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| Error::Io)?;
        file.write_all(&bytes).map_err(|_| Error::Io)?;
        file.as_file().sync_all().map_err(|_| Error::Io)?;
        file.persist(self.directory.join("trust.json"))
            .map_err(|_| Error::Io)?;
        File::open(&self.directory)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| Error::Io)?;
        Ok(())
    }
}
fn canonical_key(value: &str) -> Result<String, Error> {
    let key = PublicKey::from_openssh(value).map_err(|_| Error::InvalidStore)?;
    PublicKey::new(key.key_data().clone(), "")
        .to_openssh()
        .map_err(|_| Error::InvalidStore)
}
fn check_private(path: &Path, directory: bool) -> Result<(), Error> {
    let meta = fs::symlink_metadata(path).map_err(|_| Error::Io)?;
    if meta.file_type().is_symlink()
        || (if directory {
            !meta.is_dir()
        } else {
            !meta.is_file()
        })
        || meta.permissions().mode() & 0o077 != 0
    {
        return Err(Error::Io);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ephemeral_key() -> russh::keys::PrivateKey {
        russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519).unwrap()
    }
    fn setup() -> (tempfile::TempDir, Store) {
        let root = tempfile::tempdir().unwrap();
        let store = Store::create(&root.path().join("managed")).unwrap();
        (root, store)
    }
    fn observation(store: &Store, endpoint: Endpoint, key: &PublicKey) -> Confirmation {
        match store.inspect(endpoint, key).unwrap() {
            Inspection::Unknown(c) => c,
            _ => panic!("expected unknown"),
        }
    }
    fn accept(store: &Store, observation: Confirmation) -> Result<(), Error> {
        let fingerprint = observation.fingerprint().to_owned();
        store.confirm(observation, &fingerprint)
    }
    #[test]
    fn endpoint_scope_and_persistence_and_confirmation() {
        let (_root, store) = setup();
        let key = ephemeral_key();
        let endpoint = Endpoint::new("EXAMPLE.test", 22).unwrap();
        assert_eq!(endpoint.host(), "example.test");
        assert_eq!(endpoint.port(), 22);
        assert!(endpoint == Endpoint::new("example.test", 22).unwrap());
        let cancelled = observation(&store, endpoint.clone(), key.public_key());
        drop(cancelled);
        let c = observation(&store, endpoint.clone(), key.public_key());
        assert_eq!(
            store.confirm(c, "SHA256:incorrect"),
            Err(Error::ConfirmationMismatch)
        );
        accept(
            &store,
            observation(&store, endpoint.clone(), key.public_key()),
        )
        .unwrap();
        let reopened = Store::open(&store.directory).unwrap();
        assert!(matches!(
            reopened
                .inspect(endpoint.clone(), key.public_key())
                .unwrap(),
            Inspection::Trusted
        ));
        for other in [
            Endpoint::new("example.test", 2222).unwrap(),
            Endpoint::new("127.0.0.1", 22).unwrap(),
        ] {
            assert!(matches!(
                reopened.inspect(other, key.public_key()).unwrap(),
                Inspection::Unknown(_)
            ));
        }
        assert!(matches!(
            reopened.inspect(endpoint, ephemeral_key().public_key()),
            Err(Error::ChangedKey)
        ));
        for invalid in [
            "",
            "example.test.",
            "ssh://example.test",
            "user@host",
            "../host",
            "[::1]",
            "white space",
            "-host",
        ] {
            assert!(Endpoint::new(invalid, 22).is_err());
        }
        assert!(Endpoint::new("host", 0).is_err());
        assert!(
            Endpoint::new("2001:0db8::1", 22).unwrap() == Endpoint::new("2001:db8::1", 22).unwrap()
        );
    }
    #[test]
    fn deserialized_endpoints_cannot_bypass_validation_or_poison_storage() {
        let (_root, store) = setup();
        let key = ephemeral_key();
        for value in [
            serde_json::json!({"host":"../host", "port":22}),
            serde_json::json!({"host":"UPPER.test", "port":22}),
            serde_json::json!({"host":"host", "port":0}),
        ] {
            let endpoint = serde_json::from_value(value).unwrap();
            assert!(matches!(
                store.inspect(endpoint, key.public_key()),
                Err(Error::InvalidEndpoint)
            ));
        }
        let reopened = Store::open(&store.directory).unwrap();
        assert!(matches!(
            reopened.inspect(Endpoint::new("host", 22).unwrap(), key.public_key()),
            Ok(Inspection::Unknown(_))
        ));
    }

    #[test]
    fn malformed_missing_oversized_and_insecure_stores_fail_closed() {
        let (_root, store) = setup();
        let path = store.directory.join("trust.json");
        let key = ephemeral_key().public_key().to_openssh().unwrap();
        let entry = serde_json::json!({"endpoint":{"host":"host","port":22},"public_key":key});
        let documents = vec![b"".to_vec(),b"{}".to_vec(),b"{\"version\":1,\"version\":1,\"entries\":[]}".to_vec(),vec![b' ';MAX_BYTES as usize+1],serde_json::to_vec(&serde_json::json!({"version":1,"entries":[entry.clone(),entry]})).unwrap(), br#"{"version":1,"entries":[{"endpoint":{"host":"host","port":22},"public_key":"invalid"}]}"#.to_vec(),br#"{"version":1,"entries":[],"extra":true}"#.to_vec()];
        for bytes in documents {
            fs::write(&path, bytes).unwrap();
            assert!(Store::open(&store.directory).is_err());
        }
        fs::remove_file(&path).unwrap();
        assert!(Store::open(&store.directory).is_err());
        std::os::unix::fs::symlink("missing", &path).unwrap();
        assert!(Store::open(&store.directory).is_err());
        fs::remove_file(&path).unwrap();
        fs::write(&path, b"{}").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Store::open(&store.directory).is_err());
    }
    #[test]
    fn locked_or_deleted_store_fails_closed_after_open() {
        let (_root, store) = setup();
        let endpoint = Endpoint::new("host", 22).unwrap();
        let key = ephemeral_key();
        let lock = store.lock().unwrap();
        let start = std::time::Instant::now();
        assert!(matches!(
            store.inspect(endpoint.clone(), key.public_key()),
            Err(Error::Busy)
        ));
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
        drop(lock);
        fs::remove_file(store.directory.join("trust.json")).unwrap();
        assert!(matches!(
            store.inspect(endpoint.clone(), key.public_key()),
            Err(Error::Io)
        ));
        fs::write(store.directory.join("trust.json"), b"corrupted").unwrap();
        fs::set_permissions(
            store.directory.join("trust.json"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        assert!(matches!(
            store.inspect(endpoint, key.public_key()),
            Err(Error::InvalidStore)
        ));
    }
    #[test]
    fn concurrent_confirmations_preserve_updates_and_refuse_conflicts() {
        let (_root, store) = setup();
        let key = ephemeral_key();
        let first = Endpoint::new("first", 22).unwrap();
        let second = Endpoint::new("second", 22).unwrap();
        let c1 = observation(&store, first.clone(), key.public_key());
        let c2 = observation(&store, first.clone(), key.public_key());
        let conflict = observation(&store, first.clone(), ephemeral_key().public_key());
        let c3 = observation(&store, second.clone(), key.public_key());
        std::thread::scope(|scope| {
            let mut tasks = vec![];
            for c in [c1, c2, c3] {
                let store = &store;
                tasks.push(scope.spawn(move || accept(store, c)));
            }
            for task in tasks {
                assert_eq!(task.join().unwrap(), Ok(()));
            }
        });
        assert_eq!(accept(&store, conflict), Err(Error::ChangedKey));
        for endpoint in [first, second] {
            assert!(matches!(
                store.inspect(endpoint, key.public_key()).unwrap(),
                Inspection::Trusted
            ));
        }
        // Race two different observations: exactly one may be committed.
        let endpoint = Endpoint::new("racing", 22).unwrap();
        let a = observation(&store, endpoint.clone(), key.public_key());
        let b = observation(&store, endpoint, ephemeral_key().public_key());
        std::thread::scope(|scope| {
            let a = scope.spawn(|| accept(&store, a));
            let b = scope.spawn(|| accept(&store, b));
            let results = [a.join().unwrap(), b.join().unwrap()];
            assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
            assert_eq!(
                results
                    .iter()
                    .filter(|r| **r == Err(Error::ChangedKey))
                    .count(),
                1
            );
        });
    }
}
