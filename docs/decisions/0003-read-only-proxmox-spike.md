# ADR 0003 — A concrete read-only Proxmox client

Date: 2026-10-16
Status: Implemented against local TLS fixtures; live verification pending.

Use Reqwest with Rustls for the first provider transport. It supplies verified
TLS, timeouts and bounded streaming without shelling into the Proxmox host.
Keep this client in the server package until a proven shared boundary warrants
another crate. Expose three typed GET operations, not arbitrary API passthrough.

Reject non-HTTPS origins and redirects. Add a trusted CA when necessary; never
disable certificate or hostname verification. Mark the token header sensitive,
redact configuration Debug output and return fixed error classifications. Do
not surface raw provider errors or reqwest URLs to clients/logs.

Dev-only rcgen and tokio-rustls provide real TLS fixture servers. They verify
trust rejection and hostname mismatch, not just mocked HTTP parsing. A live
probe is a separate opt-in executable; default checks need no Proxmox host.

The user authorized autonomous sequential implementation. This does not replace
the requirement to approve concrete real-VM mutations. Live/mobile evidence
must remain explicitly pending; the full Phase 0 exit rule still applies.
