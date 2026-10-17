# Local x86 Proxmox lab preparation

## Observed host readiness

On 2026-10-17 the development host is Arch Linux x86_64, kernel 7.2.9,
AMD Ryzen 9 3900X (12 cores/24 threads), approximately 62 GiB usable RAM and
278 GiB free on the repository filesystem. The CPU exposes `svm`/AMD-V and
`/sys/module/kvm_amd/parameters/nested` returns `1`.

Outside the coding sandbox, opening `/dev/kvm` and requesting `KVM_GET_API_VERSION`
returns `12`. This proves device access, not VM execution or working nested
guests. The sandbox hides `/dev/kvm`; its absence there is not a host limitation.
No QEMU, libvirt or container runtime command was found on the inspected PATH.

The [Proxmox requirements](https://www.proxmox.com/en/products/proxmox-virtual-environment/requirements)
permit evaluation inside desktop virtualization that supports nesting. Host
observations make this a candidate; the guest must still prove its own KVM
access and execution. This removes the known M1 nesting blocker, without
claiming Proxmox has been installed or tested.

## Proposed disposable target — approval required

| Setting | Concrete proposal |
| --- | --- |
| Process/VM name | `pocketlab-pve-lab` |
| Owner | Current unprivileged development user |
| Runtime directory | `/home/moha/dev/VMAnyware/.lab/pve/` (must be ignored before creation) |
| Virtual disk | New `pve.qcow2`, 64 GiB virtual capacity; no existing disk reused |
| CPU | 8 vCPU, host CPU model with KVM, exposing AMD-V to Proxmox |
| Memory | 16 GiB |
| Installer | Official Proxmox VE amd64 ISO, exact version/hash recorded before boot |
| Install target | Only the new virtual disk; ext4/LVM evaluation layout |
| Networking | QEMU user-mode NAT; no TAP, host bridge or host firewall changes |
| Management | Host `127.0.0.1:18006` forwarded to guest TCP 8006 |
| Installer console | VNC bound only to `127.0.0.1:5901` |
| Host integration | No shared folders, physical disks, SSH keys or credential files |

Check both proposed host ports for conflicts before starting. Run QEMU as the
current user and retain its process ownership information. Package installation
requires its own privilege approval. Do not enable libvirt services merely to
run this single QEMU process. Consult the installed QEMU help and
[official invocation documentation](https://www.qemu.org/docs/master/system/invocation.html)
before selecting launch arguments.

Approval must cover package installation and creation/start of this exact outer
VM. Guest administrator credentials and dedicated API-token creation need their
own concrete approval. Do not put passwords/tokens in Git or command arguments.
Do not bypass TLS verification for the self-signed guest endpoint: establish
explicit trust before PocketLab API reads.

## Acceptance sequence after approval

1. Install the required QEMU system/image tools; record actual package versions.
   Download the selected official ISO over verified HTTPS and verify its
   published checksum before any boot. Record exact URL, version and hash.
2. Create only the approved new directory/disk and start the named VM with the
   stated limits. Verify the management/console listeners bind only to loopback.
3. Install Proxmox on its virtual disk and reboot from that disk. Record
   `pveversion`, node name, storage and bridge observations.
4. Inside Proxmox, observe AMD-V and `/dev/kvm` access. A later explicitly
   approved disposable inner guest must actually boot to prove nesting.
5. Propose the concrete template/guest VMIDs, storage, bridge, guest network,
   credentials and operations before creating them. Follow
   [template preparation](debian-template.md) only after that approval.

QEMU NAT is not a DHCP service for guests attached behind Proxmox's internal
bridge. Inner guest addressing/routing and host-to-guest SSH still need an
explicit plan; do not assume the current template guide's DHCP example works
unchanged. No LAN exposure or bridge modification is approved by this proposal.

First API evidence should be verified-TLS read-only version/nodes/template
metadata using a dedicated approved token. Clone/configure/start, snapshot,
restore and cleanup are separate operations. PocketLab has no implemented
provider mutation methods, so infrastructure readiness alone does not complete
the provisioning roadmap.

Linux frontend/server checks also do not prove Linux native Tauri: WebKitGTK
4.1 was missing during this inspection, and the application's SSH-agent/session
implementation is currently compiled only on macOS. Linux terminal support is
a separate code slice, not an automatic consequence of using x86.

## Executed outer boot — 2026-10-17

The user approved the proposed outer creation/start and installed QEMU 11.1.2.
The approved VM now runs under the development user with the table's resources.
QMP confirms KVM enabled and eight running vCPUs; visual inspection confirms
the Proxmox VE 9.2-1 ISO boot menu. Both forwarded management and VNC listeners
bind only to loopback. The installer has not been started.

ISO: `https://enterprise.proxmox.com/iso/proxmox-ve_9.2-1.iso`

Verified SHA256:
`4e88fe416df9b527624a175f24c9aa07c714d3332afb1ee3dbf3879573ef2c6c`.

Runtime files are ignored under `.lab/pve/`: `pve.qcow2`, `qemu.pid`,
`launch-args.json`, `qmp.sock`, `qemu.log` and console captures. The observed PID
was 171988; always verify current process identity rather than trusting a stale
PID. Connect a VNC viewer to `127.0.0.1:5901` for the installer console.
Port 18006 is only a forward until Proxmox is installed and serves HTTPS.

The following installation was subsequently authorized separately and completed.
Inner guests and dedicated API credentials still require concrete approval.

## Installed lab and verified HTTPS — 2026-10-17

Proxmox installed on the sole `/dev/vda` with ext4/LVM and rebooted from disk.
The node is `pve-lab` (`pve-lab.pocketlab.test`); version is
`pve-manager/9.2.2/b9984c6d90a4bd80`, kernel `7.0.2-6-pve`. Installer settings:
France, Europe/Paris, US English keyboard, lab notification placeholder
`root@pocketlab.test`. This placeholder does not provide email delivery.

The installer created guest bridge `vmbr0` over `nic0`, using `10.0.2.15/24`,
gateway `10.0.2.2` and DNS `10.0.2.3`. Host networking was not changed.
Both `local` (directory) and `local-lvm` (LVM-thin) are active. Guest `/dev/kvm`
exists and eight CPU records expose `svm`; actual nested execution is untested.

The explicitly authorized generated administrator password is stored only at
`.lab/pve/root-password` with permissions 0600. Local root login succeeded and
was then logged out. Do not include that file in logs or repository artifacts.

The public CA was exported through the owned console into
`.lab/pve/pve-root-ca.pem`. Its SHA256 fingerprint is
`D6:11:0C:E7:1B:12:F5:B6:79:22:99:22:E9:19:34:E8:98:FE:84:0D:88:69:56:70:0E:5F:A0:A6:38:5D:1D:2D`.
The following read returned HTTP 200 with certificate chain and hostname
verification, without changing system DNS or trust:

```sh
curl --fail --silent --show-error --noproxy '*' \
  --cacert .lab/pve/pve-root-ca.pem \
  --resolve pve-lab.pocketlab.test:18006:127.0.0.1 \
  --output /dev/null --write-out 'HTTP %{http_code}\n' \
  https://pve-lab.pocketlab.test:18006/
```

The VM remains running with management/VNC exposed only on host loopback.
The login page test does not establish authenticated API access. Next: prepare
a dedicated read-only account/token with concrete permissions and approval,
then perform Rust inventory reads using the explicit CA. No inner guest,
template, snapshot, restore or API token was created in this slice.

## Inventory identity preparation and subsequent approval

Concrete target: only the disposable node `pve-lab` at
`https://pve-lab.pocketlab.test:18006`, resolved to host loopback.

The prepared local script `.lab/pve/create-reader.py` would authenticate over
verified HTTPS with the lab's generated root credential, then create:

- User `pocketlab-reader@pve`, enabled, with no password, groups or ACL grants.
- Token `pocketlab-reader@pve!inventory` with `privsep=1`, no ACL grants.
- Both identities expire 24 hours after execution. Token output is stored only
  in ignored `.lab/pve/inventory-token.json`, created exclusively with mode 0600.

The script refuses an existing user/output file, never prints response bodies
or secrets, and does not automatically delete partially created resources.
The user subsequently approved and execution succeeded as recorded below.
Root tickets remained in process memory.

The schema served by this installed Proxmox at
`/pve-docs/api-viewer/apidoc.js`, fetched with verified TLS, marks GET `/version`,
`/nodes` and `/nodes/{node}/qemu` as accessible to authenticated users. The last
endpoint filters VMs by `VM.Audit` on `/vms/<vmid>`. No VM exists and no such
privilege is proposed yet. This is a minimum starting identity, not evidence
that the installed API accepts the token or that node fields will be complete.
A future concrete template needs its own scoped permission approval.

The existing Rust `proxmox_probe` compiled with `cargo build --locked -p
pocketlab-server --example proxmox_probe`. An isolated Bubblewrap mount of a
local hosts file resolves the certificate hostname to 127.0.0.1 for only the
probe process; the host's `/etc/hosts`, DNS and trust remain unchanged. Token
configuration will be passed only through the child environment, not arguments.
No live authenticated probe has run yet.

## Executed read-only inventory — 2026-10-17

After explicit approval, the prepared account and token were created without
ACLs. Both expire at **2026-10-18 21:22:17 UTC**. Administrative readback
confirmed privilege separation, matching expiry and no direct ACL grants.
The token's effective permission listing is empty. Secret output is ignored
and mode 0600; it is not recorded here.

The compiled Rust probe returned Proxmox 9.2.2 (release 9.2) and node pve-lab
online. Direct token GETs confirmed empty visible VM inventory and permissions.
Live negative checks rejected a bad token (401), an untrusted CA and a wrong
TLS hostname. The Rust probe also returned its fixed safe errors for a bad
token and absent lab CA. No write endpoint was called with this token.

No guest/template exists yet. Next: prepare a concrete Debian template VMID,
image, storage and networking plan for approval, then grant only its scoped
VM.Audit visibility to the account and token. Do not broaden to a global role
merely to obtain a nonempty inventory.
