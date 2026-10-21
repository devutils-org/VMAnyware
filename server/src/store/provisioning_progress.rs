//! One read snapshot of the retained journal, projected to public progress fields.
use super::{operations, provisioning::read, ProvisioningError, Store};
use crate::provisioning_intent::Progress;
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}
impl Store {
    pub fn provisioning_progress(&self, id: &str) -> Result<Option<Progress>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection.transaction().map_err(unavailable)?;
        let operation = operations::read(&tx, id).map_err(unavailable)?;
        if !operation.is_some_and(|o| o.kind == "vm_provision") {
            tx.commit().map_err(unavailable)?;
            return Ok(None);
        }
        let job = read(&tx, id)?.ok_or(ProvisioningError::Unavailable)?;
        let progress = job.progress().ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(Some(progress))
    }
}
