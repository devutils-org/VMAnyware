# Read-only VM power observation

`GET /api/v1/provider/nodes/{node}/vms/{vmid}/status` observes one explicitly
addressed QEMU VM through the configured verified-TLS Proxmox client. It makes
one provider GET to `/nodes/{node}/qemu/{vmid}/status/current`. No inventory scan,
provider write, guest command, durable operation or managed-instance registration
is performed. The observation is not a reservation or authorization to mutate.

Example response:

```json
{
  "node": "pve-lab",
  "vmid": 202,
  "status": "running",
  "qmpStatus": "paused"
}
```

This is a contract example, not live-host evidence. `status` is `running` or
`stopped`. It describes provider power state, not guest readiness. In particular,
a frozen VM may have `status: running` and `qmpStatus: prelaunch`. Even both
fields reading `running` proves neither Cloud-Init completion nor usable SSH.

`qmpStatus` is `running`, `stopped`, `paused`, `prelaunch`, `shutdown`, `suspended`,
`guest-panicked`, `unknown`, or null when absent. Other provider QEMU states are
normalized to `unknown`; raw strings and unrelated provider fields are omitted.
The provider VMID must equal the requested VMID. Missing/malformed identity or
power state fails the read. The response node identifies the addressed API path;
it is not independent proof against node migration or VMID reuse.

## Validation and errors

Node names are 1–63 ASCII characters, start with a letter/digit and contain only
letters, digits, dots, underscores or hyphens. VMIDs use canonical decimal form
in 100–999999999. Signs, leading zeros, path separators and malformed path data
are rejected with HTTP 400 `invalid_vm_target` before contacting the provider.

Without a provider, a valid request returns HTTP 503 `provider_not_configured`.
Provider authentication/permission failures return HTTP 502 with
`provider_unauthorized`/`provider_forbidden`. A provider timeout returns HTTP 504
`provider_timeout`; remaining failures, including upstream missing targets and
invalid responses, return HTTP 502 `provider_unavailable`. Missing targets are
not asserted to be deleted, since a failed provider read is inconclusive.
Errors use the standard fixed JSON envelope; upstream bodies are never returned.

The existing provider deadline (15 seconds), verified hostname/CA, redirect
refusal and 2 MiB response bound apply. The local service remains loopback-only.
POST and mutation methods are unsupported. Logs contain the static route pattern,
never the requested node, VMID, query string or credentials.

## Permissions and evidence

The [upstream Proxmox QEMU API implementation](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
declares `VM.Audit` on `/vms/{vmid}` for GET status/current. This is a documented
requirement, not newly tested least-privilege evidence on the installed lab.
No token/ACL changes are included. Any real-host check remains opt-in against an
explicitly configured disposable target.

Local generated-certificate HTTPS fixtures verify the exact GET path, power/QMP
separation, absent/unknown QMP data, identity rejection, untrusted CA refusal,
authentication/permission errors, bounded timeout and redaction. API tests cover
invalid targets and method refusal; real subprocess tests verify static route
logs and request IDs. No live Proxmox request or native-client walkthrough is
claimed for this endpoint. Persistence, ownership and lifecycle execution remain
separate work before start/stop operations can be exposed safely.
