# Read-only Proxmox feasibility probe

No real host has been tested yet. Local TLS fixture tests verify transport,
certificate trust/hostname checks, authentication/error mapping and parsing.

## Scope and permissions

Only `GET /api2/json/version`, `GET /api2/json/nodes` and
`GET /api2/json/nodes/{node}/qemu` are implemented. The last route returns only
VMs for which the effective identity has `VM.Audit` on `/vms/<vmid>` according to
the official API schema. An empty inventory does not prove that no VMs exist.

Use a dedicated account/token with privilege separation and audit access only
to the test template/VMs. Effective token privileges are constrained by both
the user and the token ACLs. Actual ACL behavior must be verified on the target
Proxmox version; no administrator token is needed for these reads.

This probe cannot create, start, stop, snapshot, restore or delete a VM.

## Runtime configuration

Set the following in your local shell or service environment. Do not paste
credentials into the chat, YAML definitions or command-line arguments. The
application does not automatically load `.env` files.

| Variable | Meaning |
| --- | --- |
| `POCKETLAB_PROXMOX_URL` | HTTPS origin, e.g. `https://pve.example:8006`, without API suffix, user info, query or fragment |
| `POCKETLAB_PROXMOX_TOKEN_ID` | Dedicated token identifier, e.g. `reader@pve!pocketlab` |
| `POCKETLAB_PROXMOX_TOKEN_SECRET` | Token secret, kept only in the control-plane process |
| `POCKETLAB_PROXMOX_CA` | Optional path to a PEM CA certificate trusted for this server |

For a cluster CA, obtain its **public CA certificate** through a trusted channel
and use a hostname present in the server certificate. Do not bypass verification.
Proxy environment variables are intentionally ignored for this LAN/VPN client.
The request deadline is 15 seconds and the response limit is 2 MiB. Redirects
are refused so credentials cannot be forwarded to a different destination.

After explicitly selecting a test endpoint and configuring these variables:

```sh
POCKETLAB_PROXMOX_PROBE=read-only cargo run -p pocketlab-server --example proxmox_probe
```

The extra opt-in prevents accidental live probing. Default tests use generated
short-lived TLS identities on localhost and never use runtime credentials.

The probe reports version, node status and templates visible to the token.
It emits fixed errors rather than raw HTTP bodies, transport URLs or headers.
Template VMIDs/versions must still be mapped explicitly to PocketLab image names.

## Evidence required before provisioning

Record the Proxmox version, certificate trust method, tested ACLs, Debian
template preparation, target storage/bridge and disposable VMID allocation.
Get explicit approval for the concrete clone/configure/start/cleanup operations.
Real provisioning, snapshots and readiness remain unverified until those tests.

References: [Proxmox API viewer](https://pve.proxmox.com/pve-docs/api-viewer/),
[API schema data](https://pve.proxmox.com/pve-docs/api-viewer/apidoc.js),
[API documentation](https://pve.proxmox.com/wiki/Proxmox_VE_API),
[Reqwest TLS configuration](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html).
