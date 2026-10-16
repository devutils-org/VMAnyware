use std::{
    fs::{File, OpenOptions},
    path::Path,
};

/// Keep this handle alive for the service lifetime. Never unlink a held lock.
pub struct ServiceLock {
    _file: Option<File>,
}

impl ServiceLock {
    pub fn acquire(database: &Path) -> Result<Self, &'static str> {
        if database == Path::new(":memory:") {
            return Ok(Self { _file: None });
        }
        let mut path = database.as_os_str().to_os_string();
        path.push(".lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|_| "Cannot open the database service lock.")?;
        file.try_lock()
            .map_err(|_| "Another service owns this database, or locking is unavailable.")?;
        Ok(Self { _file: Some(file) })
    }
}
