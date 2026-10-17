# Prepare a Debian 13 Cloud-Init template

Status: documentation checked against official sources on 2026-10-16. The
unbooted preparation path was executed on the disposable nested x86 Proxmox
lab on 2026-10-18; see [exact target and evidence](x86-template.md). A separate
[static-network clone smoke](x86-clone.md) booted and verified real SSH/resources;
Cloud-Init completed with a deprecation warning, so clean readiness remains open.
VMAnyware currently previews and checks plans; it cannot execute provisioning.

Run the host commands manually in Bash on the selected Proxmox node, not on the
Mac running VMAnyware. Creating/importing/resizing/converting a VM changes that
host. Before execution, select the concrete node, unused template VMID, storage
and bridge and approve those operations. Preparing this document authorizes no
remote operations. The optional clone test needs a separately selected VMID.
Do not paste the entire guide as one script or rerun mutation steps blindly.

## 1. Establish the disposable target

Install Proxmox VE using its official installation guide. The installer can
erase the selected disk; choose a dedicated test disk and preserve needed data
before installing. Enable hardware virtualization in the PC firmware and use a
LAN/VPN management address reachable from the Mac. Match the image architecture
to the host; this guide's image is specifically amd64.

Record the following before creating anything:

| Input | Example only | Required observation |
| --- | --- | --- |
| Proxmox node | `pve-test` | Exact name shown by Proxmox |
| Template VMID | `9000` | Unused across the cluster, including containers |
| Storage | `local-lvm` | Enabled, active, allows VM images, sufficient free space |
| Bridge | `vmbr0` | Active bridge connected to the intended guest network |
| Guest network | IPv4 DHCP | A DHCP server and routing actually exist on that network |
| Template disk | `scsi0`, 8 GiB | Imported image is no larger; expansion only |
| Guest user | `debian` | Used consistently in Cloud-Init and the catalog |

A bridge does not provide a DHCP server automatically. The storage/bridge names
are examples, not defaults VMAnyware discovers. On the selected node, inspect:

```sh
pveversion
hostname
pvesm status
pvesh get /nodes
pvesh get /cluster/nextid --vmid 9000
ip -brief link show vmbr0
```

Replace the IDs/names before running even the read-only commands. A successful
next-ID assertion observes availability only at that moment; it reserves nothing.
Review the storage's allowed content and bridge configuration in the Proxmox UI.
Also inspect the locally installed `qm help create`, `qm help set`,
`qm help disk resize`, `qm help template` and `qm help clone`: the installed
version is authoritative if it differs from the online manual.

## 2. Download and record a fixed cloud image

Use Debian's `genericcloud` image, which is intended for virtualized cloud
hardware. Use the cloud image with Cloud-Init, rather than an installation ISO
or the `nocloud` image variant. The dated build below was present in the official
index when this guide was written. It is an example pin, not a recommendation to
retain that build indefinitely. Choose a reviewed newer dated build when needed
and change the directory, filenames and catalog revision together.

In a new working directory on the Proxmox node:

```sh
mkdir -p /var/lib/vz/vmanyware-images/debian-13-20261015-2632
cd /var/lib/vz/vmanyware-images/debian-13-20261015-2632
IMAGE_BUILD=20261015-2632
IMAGE_FILE="debian-13-genericcloud-amd64-${IMAGE_BUILD}.qcow2"
IMAGE_BASE="https://cloud.debian.org/images/cloud/trixie/${IMAGE_BUILD}"
curl --fail --show-error --location --proto '=https' --proto-redir '=https' \
  --output "$IMAGE_FILE" "$IMAGE_BASE/$IMAGE_FILE"
curl --fail --show-error --location --proto '=https' --proto-redir '=https' \
  --output SHA512SUMS "$IMAGE_BASE/SHA512SUMS"
curl --fail --show-error --location --proto '=https' --proto-redir '=https' \
  --output image-build.json "$IMAGE_BASE/debian-13-genericcloud-amd64-${IMAGE_BUILD}.json"
sha512sum --check --ignore-missing SHA512SUMS
qemu-img info "$IMAGE_FILE"
```

Run each command only after the preceding one succeeds. Require an explicit
`<image filename>: OK` result and exit status zero from the checksum check.
Confirm qcow2 format and a virtual disk size no larger than 8 GiB before import.
Never shrink an image to make it fit this example. If it is larger, choose a
larger whole-GiB template disk and update `sourceDiskGiB` consistently.

Keep the dated URL, filename, SHA-512 manifest, matching image hash, package/build
metadata and Proxmox version as local preparation evidence. HTTPS and a matching
manifest verify transport and file integrity; this is not a separate signature
verification of the publisher. This directory did not advertise a detached
manifest signature when checked. Do not invent a signature verification result.
The `latest` URL can change and should not identify a reproducible template.

## 3. Import without booting the template

The following commands assume you have explicitly selected and approved
VMID **9000**, **local-lvm** storage and bridge **vmbr0** on this node. Substitute
your actual choices consistently. Stop immediately after any failure; inspect
the VM and storage state before retrying. Do not overwrite an existing VM or
remove an existing disk to make a retry succeed.

From the image working directory, with `IMAGE_FILE` still set:

```sh
pvesh get /cluster/nextid --vmid 9000
qm create 9000 --name vmanyware-debian-13-20261015 \
  --memory 2048 --cores 2 --ostype l26 \
  --scsihw virtio-scsi-pci --net0 virtio,bridge=vmbr0
qm set 9000 --scsi0 "local-lvm:0,import-from=$(pwd)/$IMAGE_FILE"
qm disk resize 9000 scsi0 8G
qm set 9000 --ide2 local-lvm:cloudinit
qm set 9000 --boot order=scsi0 --serial0 socket --vga serial0
qm set 9000 --ciuser debian --citype nocloud --ciupgrade 0 --ipconfig0 ip=dhcp
qm config 9000 --current 1
qm status 9000
```

The resize argument is the absolute final size, not `+8G`. Skip resize if the
imported disk is already exactly 8 GiB; Proxmox may reject a no-op resize. Verify
the actual imported size before deciding. Disk shrinking is unsupported.

Before conversion, inspect these exact properties:

- VM is stopped and has never been started, so no per-instance Cloud-Init state,
  SSH host keys or test identity has been generated by a guest boot.
- `scsi0` references the imported disk and reports `size=8G` (or your chosen size).
- `ide2` is the Cloud-Init drive; the generated volume normally has the basename
  `vm-9000-cloudinit`, possibly with a storage-specific path or extension.
- Boot order selects `scsi0`; SCSI controller is `virtio-scsi-pci`; serial console
  is configured; `net0` uses the intended bridge.
- Cloud-Init uses `debian`, NoCloud and IPv4 DHCP. `ciupgrade=0` disables the
  Proxmox-requested automatic upgrade; it does not freeze future package repos.
- No password, personal SSH key, custom Cloud-Init snippet, host passthrough,
  hook script or unintended extra disk is configured.

Do not enable the QEMU guest agent unless its installation and use have actually
been validated. A Cloud-Init disk alone does not prove guest Cloud-Init works.
Convert the still-unbooted VM only after reviewing the configuration:

```sh
qm template 9000
qm config 9000 --current 1
```

Require `template: 1`, the expected disk size and the retained Cloud-Init drive.
Record a local template revision including image build and preparation revision,
for example `debian-13-amd64-20261015-2632-r1`. If any preparation changes, use a
new revision and record them. The revision is an operator label, not a verified
image digest. Never start the template to validate it; validate a disposable clone.

## 4. Optional manual clone smoke test

The local x86 lab now preserves a stopped clone 201; do not replay these example
commands against that existing VM. Its actual isolated-network test differs
from the DHCP example below and is recorded in docs/x86-clone.md.

This section performs real mutations. First approve the concrete source/target,
full clone, configuration, start and eventual cleanup independently. Example
clone **201** must be unused and disposable. It is not managed by VMAnyware;
a successful manual test does not demonstrate application provisioning.

Copy only your intended **public** OpenSSH key to
`/root/vmanyware-test.pub` on the host through a trusted channel. No private key
belongs on the Proxmox host. Check its public fingerprint before use.
Then, after substituting the approved IDs and storage:

```sh
pvesh get /cluster/nextid --vmid 201
qm clone 9000 201 --name vmanyware-manual-smoke --full 1 --storage local-lvm
qm set 201 --cores 2 --memory 2048 --ciuser debian \
  --sshkeys /root/vmanyware-test.pub --ipconfig0 ip=dhcp
qm disk resize 201 scsi0 12G
qm cloudinit dump 201 user
qm cloudinit dump 201 network
```

Wait for each CLI command to finish successfully before continuing. Inspect the
generated Cloud-Init user/network data before start. Open the Proxmox UI serial
console for clone 201, ready to capture its first boot output, then start it:

```sh
qm start 201
```

Observe boot and find the address through the DHCP lease table or trusted guest
console. There is no guest agent or automatic address discovery assumed here.
A successful start command alone is not guest readiness.

Capture the guest's public SSH host-key fingerprint in the trusted Proxmox
serial boot output. Cloud-Init normally emits host public keys and fingerprints
to the console; verify which key type the SSH prompt presents. Open the serial
console before the first boot so this output is not missed. If you can already
log in through a trusted console, `ssh-keygen -lf
/etc/ssh/ssh_host_ed25519_key.pub` also reports the Ed25519 public fingerprint.
The cloud image may not allow password console login, so do not rely on that
route. If fingerprints are not emitted and no trusted login/verification channel
is available, stop and establish one before accepting the SSH key. Connect from your client using your existing local SSH
identity and `ssh debian@<guest-address>`; compare the first-use fingerprint to
the trusted observation. Do not disable host verification or accept a changed
host key without investigating it.

Inside the clone, record:

```sh
cloud-init status --wait --long
cat /etc/os-release
id
nproc
free -m
lsblk
findmnt /
```

Require Cloud-Init completion without errors, Debian 13, the intended public-key
login/user, 2 guest CPUs, approximately 2 GiB RAM (allowing guest overhead), a
12 GiB virtual disk and expanded root filesystem. `df -h /` can corroborate
filesystem capacity; disk growth alone does not prove filesystem growth. If
Cloud-Init fails, collect relevant guest diagnostics locally; do not publish
raw configuration/logs containing identity or secret material. Package installs,
snapshots, restores and VMAnyware SSH/PTY remain separate, unverified tests.

Stop the test after collecting evidence. Use the Proxmox UI to gracefully shut
down the approved clone and verify its identity before any separately approved
removal of that clone and its disks. No automatic destroy/cleanup command is
provided. Preserve the template and reusable definition. If a command failed
partway, record the VMID and disks that exist before deciding how to clean up.

## 5. Configure VMAnyware and run read-only checks

On the Mac/control-plane machine, copy `examples/templates.yaml` to a local file
outside the repository and replace every example value using your recorded
metadata. For the example preparation above the mapping would be:

```yaml
apiVersion: vmanyware.dev/v1
kind: TemplateCatalog
images:
  - image: debian-13
    node: pve-test
    templateVmid: 9000
    templateVersion: debian-13-amd64-20261015-2632-r1
    storage: local-lvm
    bridge: vmbr0
    diskDevice: scsi0
    sourceDiskGiB: 8
    guestUser: debian
```

Configure trusted TLS and a dedicated read-only token using
[Proxmox setup](proxmox-setup.md). Never put the token in this YAML or in chat.
Set `VMANYWARE_TEMPLATES` to the local catalog path and restart `make server`.
The server does not load dotenv files automatically. Approve the concrete test
endpoint for read-only access, then use the opt-in inventory probe or native
**Connect**, **Preview provisioning**, and **Check against Proxmox** actions.
The current desktop control plane remains on the Mac's loopback interface;
there is no need to expose port 8080 to the LAN. Only its Proxmox API client
needs access to the PC over verified HTTPS.

Use a new unused requested VMID, not 9000 or the smoke-test clone's ID. If using
`examples/kernel-lab.yaml`, replace the demonstration public key with your own
public key and size CPU/memory to fit the PC; its 8 GiB RAM request may be too
large for a small lab. Keep disk size at least `sourceDiskGiB`.

Require the expected source/target and matching catalog revision in the preview.
Review every preflight result. Storage can be invisible to a restricted token;
unknown does not mean available. The current checker recognizes standard
Cloud-Init volume names and integer disk sizes only. Revision identity, DHCP,
guest readiness and execution permissions remain unknown even when observable
checks pass. No VMID is reserved and no VM creation action is enabled.

Record the tested Proxmox version, template revision/image hash, node/VMID,
storage/bridge, trusted CA method, effective read-only ACLs and actual results.
Keep credentials and private material out of the repository. Only then can the
live read-only feasibility milestone be assessed; provisioning acceptance still
requires implementing and testing application execution with concrete approval.

## Official references

- [Proxmox installation](https://pve.proxmox.com/pve-docs/chapter-pve-installation.html)
- [Proxmox Cloud-Init support](https://pve.proxmox.com/wiki/Cloud-Init_Support)
- [Proxmox `qm` manual](https://pve.proxmox.com/pve-docs/qm.1.html)
- [Cloud-Init SSH console output](https://docs.cloud-init.io/en/latest/reference/yaml_examples/ssh.html)
- [Debian cloud image index](https://cloud.debian.org/images/cloud/trixie/)
- [Dated build and checksum manifest](https://cloud.debian.org/images/cloud/trixie/20261015-2632/)

The online sources establish available syntax/artifacts, not execution evidence
on the user's hardware. `STATE.md` records the actual verification boundary.

The fresh clone 202 [compatibility retest](cloud-init-compatibility.md) proves
clean Cloud-Init first boot with the target-specific mapped-user snippet.
The original 201 degraded result remains diagnostic evidence; this recipe
requires explicit snippet ownership and regeneration when its inputs change.
