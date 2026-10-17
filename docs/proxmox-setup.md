# Read-only Proxmox feasibility probe

The disposable local Proxmox lab has passed the Rust version/node inventory
probe with verified TLS and a dedicated token. Local TLS fixture tests also
verify transport, certificate trust/hostname checks, authentication/error mapping
and parsing. Stopped Debian template 9000 has now been inspected with a
VM.Audit-only role scoped to that VM; see [execution evidence](x86-template.md).

## Scope and permissions

The inventory probe uses `GET /api2/json/version`, `GET /api2/json/nodes` and
`GET /api2/json/nodes/{node}/qemu`. The last route returns only
VMs for which the effective identity has `VM.Audit` on `/vms/<vmid>` according to
the official API schema. An empty inventory does not prove that no VMs exist.

Use a dedicated account/token with privilege separation and audit access only
to the test template/VMs. Effective token privileges are constrained by both
the user and the token ACLs. Actual ACL behavior must be verified on the target
Proxmox version; no administrator token is needed for these reads.

The application also offers opt-in read-only [provisioning checks](provisioning-plan.md):

| GET endpoint (under `/api2/json`) | Purpose and documented visibility |
| --- | --- |
| `/nodes/{node}/qemu/{vmid}/config?current=1` | Current source configuration; `VM.Audit` on `/vms/{vmid}` |
| `/nodes/{node}/storage` | Status visible with `Datastore.Audit` or `Datastore.AllocateSpace` on the storage |
| `/nodes/{node}/network` | Network list; API schema permits authenticated users |
| `/cluster/nextid?vmid=<candidate>` | Assert candidate availability at observation time; API schema permits authenticated users |

These are documented read requirements, not a tested minimum provisioning role.
Storage hidden by ACLs remains unknown. Provider failures and incomplete metadata
remain unknown. Actual token behavior must be verified on the disposable host.
No read establishes permission to clone, configure, start or snapshot a VM.

Neither the probe nor the preflight can create, start, stop, snapshot, restore or
delete a VM.

## Runtime configuration

Set the following in your local shell or service environment. Do not paste
credentials into the chat, YAML definitions or command-line arguments. The
application does not automatically load `.env` files.

| Variable | Meaning |
| --- | --- |
| `VMANYWARE_PROXMOX_URL` | HTTPS origin, e.g. `https://pve.example:8006`, without API suffix, user info, query or fragment |
| `VMANYWARE_PROXMOX_TOKEN_ID` | Dedicated token identifier, e.g. `reader@pve!vmanyware` |
| `VMANYWARE_PROXMOX_TOKEN_SECRET` | Token secret, kept only in the control-plane process |
| `VMANYWARE_PROXMOX_CA` | Optional path to a PEM CA certificate trusted for this server |

For a cluster CA, obtain its **public CA certificate** through a trusted channel
and use a hostname present in the server certificate. Do not bypass verification.
Proxy environment variables are intentionally ignored for this LAN/VPN client.
The request deadline is 15 seconds and the response limit is 2 MiB. Redirects
are refused so credentials cannot be forwarded to a different destination.

After explicitly selecting a test endpoint and configuring these variables:

```sh
VMANYWARE_PROXMOX_PROBE=read-only cargo run -p vmanyware-server --example proxmox_probe
```

The extra opt-in prevents accidental live probing. Default tests use generated
short-lived TLS identities on localhost and never use runtime credentials.

The probe reports version, node status and templates visible to the token.
It emits fixed errors rather than raw HTTP bodies, transport URLs or headers.
Template VMIDs/versions must still be mapped explicitly to VMAnyware image names.

## Preparing the template

Use the [Debian 13 template procedure](debian-template.md) to choose a dated
cloud image, verify its checksum, import an unbooted template and record the
explicit mapping. The procedure is documentation, not real-host test evidence.

## Evidence required before provisioning

Record the Proxmox version, certificate trust method, tested ACLs, Debian
template preparation, target storage/bridge and disposable VMID allocation.
Get explicit approval for the concrete clone/configure/start/cleanup operations.
Real provisioning, snapshots and readiness remain unverified until those tests.

References: [Proxmox API viewer](https://pve.proxmox.com/pve-docs/api-viewer/),
[API schema data](https://pve.proxmox.com/pve-docs/api-viewer/apidoc.js),
[API documentation](https://pve.proxmox.com/wiki/Proxmox_VE_API),
[Reqwest TLS configuration](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html).

## Disposable lab evidence — 2026-10-17

Target: only `pve-lab` in the approved local QEMU VM, Proxmox 9.2.2.
The existing Rust example returned version 9.2.2/release 9.2 and
`Node pve-lab: online`, with no visible templates. Separate authenticated
GETs returned HTTP 200 for `/version`, `/nodes`, `/nodes/pve-lab/qemu`
(empty array) and `/access/permissions` (empty object).

The approved account is `vmanyware-reader@pve`, created without a password,
group or ACL grant. Token `inventory` uses privilege separation and has no ACL
grant. Administrative readback confirms these settings and matching account/
token expiry: **2026-10-18 21:22:17 UTC**. This expiry applies only to the lab
identity; renewal/replacement is a credential change requiring approval.

The token secret remains in ignored `.lab/pve/inventory-token.json`, mode 0600,
and is passed only in the probe child environment. The root credential was
used over verified HTTPS only for approved identity creation and administrative
metadata readback; it was not passed to the Rust probe. No credentials or
response bodies containing them were printed.

The lab CA is explicitly trusted. Bubblewrap supplies a process-local hosts
file mapping `pve-lab.vmanyware.test` to 127.0.0.1 for port 18006; host DNS,
`/etc/hosts` and system trust are unchanged. Separate live checks rejected
an invalid token with HTTP 401, an untrusted CA and a wrong TLS hostname.
The Rust probe also emitted the expected fixed errors for an invalid token
and missing lab CA. No mutation request was used as a negative permission test.

These results establish the minimal starting identity for an empty inventory.
They do not prove template visibility, per-VM `VM.Audit`, storage audit access
or provisioning permissions. The phase-0 known-template gate remains open.
See [local lab details](x86-lab.md).

On 2026-10-18 the approved reader role was extended only by VM.Audit at
`/vms/9000` for both user and token, nonpropagating. Template list/current-config
GETs and the Rust inventory probe succeeded for the known stopped template.
This supersedes the empty-inventory/empty-permission starting state above;
account/token expiry is unchanged. No storage audit or mutation privilege was
granted. Guest readiness and live provisioning preflight remain unverified.
