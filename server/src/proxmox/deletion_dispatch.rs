//! One fixed VM DELETE. Call only after the durable dispatch marker is committed.
use super::{header, Error, Proxmox, TaskReference, VmReference};
impl Proxmox {
    /// No redirects, retries, lock bypass, purge or unreferenced-volume destruction.
    /// Any error except explicit authentication/authorization refusal is ambiguous.
    pub async fn dispatch_deletion(&self, target: &VmReference) -> Result<TaskReference, Error> {
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .map_err(|_| Error::Configuration)?
            .extend([
                "api2",
                "json",
                "nodes",
                target.node(),
                "qemu",
                &target.vmid().to_string(),
            ]);
        url.query_pairs_mut()
            .append_pair("purge", "0")
            .append_pair("destroy-unreferenced-disks", "0");
        let response = self
            .client
            .delete(url)
            .header(header::AUTHORIZATION, self.authorization.clone())
            .send()
            .await
            .map_err(super::transport_error)?;
        let upid: String = Self::decode_response(response).await?;
        let task = TaskReference::new(target.node(), &upid).map_err(|_| Error::InvalidResponse)?;
        if !task.matches_deletion(target) {
            return Err(Error::InvalidResponse);
        }
        Ok(task)
    }
}
