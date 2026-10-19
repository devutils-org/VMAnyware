//! Fixed lifecycle transport. Call only after committing the durable dispatch guard.
use super::{header, Error, Proxmox, TaskReference, VmReference};
use crate::lifecycle::Action;

impl Proxmox {
    /// Submit once, with no redirects or transport retries. An error other than
    /// Unauthorized/Forbidden cannot establish that the action was not accepted.
    /// This transport does not grant approval or perform precondition checks;
    /// the execution worker must commit its dispatch marker before calling it.
    pub async fn dispatch_lifecycle(
        &self,
        target: &VmReference,
        action: Action,
    ) -> Result<TaskReference, Error> {
        let (suffix, body) = match action {
            Action::Start => ("start", ""),
            Action::Shutdown => ("shutdown", "timeout=60&forceStop=0"),
        };
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
                "status",
                suffix,
            ]);
        let response = self
            .client
            .post(url)
            .header(header::AUTHORIZATION, self.authorization.clone())
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(super::transport_error)?;
        let upid: String = Self::decode_response(response).await?;
        let task = TaskReference::new(target.node(), &upid).map_err(|_| Error::InvalidResponse)?;
        if !task.matches_action(target, action) {
            return Err(Error::InvalidResponse);
        }
        Ok(task)
    }
}
