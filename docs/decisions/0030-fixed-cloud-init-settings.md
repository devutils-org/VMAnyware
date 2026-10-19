# ADR 0030 — Fixed network and passwordless NoCloud settings

Date: 2026-10-18
Status: Transport/settings observation tested locally; durable storage/executor implemented locally; seed compatibility pending.

Bind the validated definition hash/name to the retained plan before constructing
Cloud-Init inputs. The caller also supplies the exact observed primary volume,
configuration revision and net0 configuration. No production worker/API dispatches
this transport. It requires the dedicated durable stage marker from ADR 0031 before PUT.

The supported network is one virtio net0 with a valid unicast nonzero MAC,
bridge and optional firewall flag. Preserve its MAC and optional firewall setting;
replace only the bridge with the catalog bridge. Reject extra interfaces/IP configs,
VLAN/trunk/rate/MTU or other unmodeled properties instead of discarding them. The
mapped primary disk must already have the requested exact size on the retained
volume. Require the exact target-specific ide2 Cloud-Init volume on mapped storage,
stopped clone identity, current CPU/memory, matching revisions and no pending
changes. Existing template/installed-provider layouts outside this narrow contract
remain unsupported until independently tested.

Use the synchronous QEMU config PUT with only digest, net0, ipconfig0=ip=dhcp,
citype=nocloud, catalog ciuser, ciupgrade=0, canonical public sshkeys and the fixed
delete list cicustom,cipassword. Removing inherited custom overrides/passwords
prevents silently provisioning another user's seed or credential. Disabling
implicit package upgrades leaves package execution to the declared package stage.
No arbitrary fields, commands, password, extra network or custom snippet path can
be supplied through this transport.

The [upstream QEMU API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
and [Cloud-Init generator](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuServer/Cloudinit.pm)
URI-unescape stored SSH keys. URI-encode the canonical key lines internally, then
form-encode the complete request. Strip comments with the existing SSH parser to
preserve key material without injecting comment syntax into generated YAML. The
retained definition/hash still includes the original values; no stored definition
is rewritten. Debug and observation results do not contain key material. No new
dependency is needed for these encodings or key validation.

The existing verified-TLS, no-redirect/no-retry client sends at most one PUT within
a 20-second operation deadline. An independent bounded GET observation requires
exact applied network/user/key/settings, absence of inherited overrides/password,
consistent revision, exact primary disk and valid target-specific Cloud-Init drive.
It returns only key count, safe target/settings evidence and guestReady=false.
This establishes neither regenerated seed contents nor clean first boot.

Important compatibility boundary: the known Proxmox/Cloud-Init 25.1.4 scalar-user
deprecation in ADR 0017 is not fixed by these API settings. This transport uses
built-in generation and clears inherited snippets; it cannot replace the approved
lab's target-specific corrected snippet recipe. Product seed compatibility and
snippet ownership/lifecycle still need implementation before claiming support for
that pinned combination. Keep strict exit-zero/no-recoverable-errors readiness;
never weaken it or claim a configured VM is ready to bypass this work.

Four targeted local HTTPS tests pass: exact fixed form/definition binding,
pre-PUT disk/drive/network/revision/resource/identity gates and no replay after
lost/malformed replies, independent exact applied settings with readiness false,
and multiple public keys preserved through both encoding layers. Scoped library
Clippy passes. No schema/dependency, real infrastructure or credential change is
part of this slice.

See [ADR 0031](0031-durable-cloud-init-stage.md) for schema-9 storage and recovery guards.
