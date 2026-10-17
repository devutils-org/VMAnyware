# Proposed first nested Debian clone and SSH smoke

Status: approved and executed on 2026-10-18. Clone 201 booted and passed
verified OpenSSH/resource checks, then shut down cleanly. Cloud-Init completed
with a deprecation warning; clean readiness remains unresolved. The existing stopped [template 9000](x86-template.md) is the
source. This plan intentionally tests guest execution before application
provisioning, whose mutation methods are not implemented.

## Concrete approval scope

| Resource | Proposed target/operation |
| --- | --- |
| Proxmox | Only disposable pve-lab inside vmanyware-pve-lab |
| Source | Full clone from stopped template 9000; never start or modify template |
| Clone | New VMID 201, name vmanyware-debian-smoke-201 |
| Compute/disk | 2 vCPU, 2048 MiB RAM; full copy on local-lvm, expand scsi0 from 8 to 12 GiB |
| Private bridge | New runtime-only vmbr1, 192.0.2.1/30, no physical port |
| Guest NIC | Only vmbr1; static 192.0.2.2/30, no default gateway; DNS not required |
| Guest identity | debian, one new disposable Ed25519 public key via Cloud-Init; no password |
| Key ownership | New ignored .lab/pve/clone201 identity, mode 0600; no personal keys/config/agent |
| SSH relay | Bounded owned Python process inside pve-lab, listen 10.0.2.15:18222, connect only 192.0.2.2:22 |
| Outer QEMU | Add runtime hostfwd from 127.0.0.1:18222 to 10.0.2.15:18222 through owned QMP |
| First boot | Start with CPU frozen, attach serial recorder, then continue CPU for first boot |
| Readiness | Verify Cloud-Init, guest OS/resources and authenticated SSH separately from provider task success |
| End of test | Gracefully shut down only 201; close owned relay/serial recorder, remove added forward and unused runtime bridge |

No Linux-host network/firewall change, Proxmox management-bridge change, NAT,
DHCP service, package installation, guest-agent installation, new API token,
ACL expansion, snapshot, restore or VM/disk deletion is included. The clone
and generated identity remain as owned local artifacts; never force-stop an
unresponsive guest or delete it without a separate concrete decision.

## Observations and limits

Verified read-only before preparation: VMID 201 available, vmbr1 absent from
Proxmox's network list, template 9000 stopped and local-lvm enabled/active with
20,301,635,126 bytes available. Host port 18222 has no listener. Host routes do
not contain 192.0.2.0/30. These are observations, not reservations. Recheck all
of them, including routes/interfaces/listeners inside pve-lab, before mutation.
Refuse collisions or unexpected existing resources; do not overwrite them.

The installed qm manual supports `freeze` (CPU frozen at startup). The installed
outer QEMU monitor documents hostfwd_add, and current info usernet shows only
the existing management forward on 18006. No forwarding mutation has been made.
The exact serial socket must be observed after starting frozen rather than
assuming first-boot output can be recovered later.

The relay gives this host access to guest SSH without routing the private guest
subnet. No default route is configured in the guest. Proxmox may inherit host DNS
settings into Cloud-Init; they are not assumed reachable or required for literal-
address SSH. Internet/package operations
are outside the acceptance criteria. This is a test topology, not a production
network or an adversarial guest-isolation guarantee. The runtime bridge does not
survive an outer reboot. The preserved clone must not be restarted until its
required test network is deliberately restored.

The current product preview always declares DHCP and cannot represent this
static smoke network. Do not claim that this manually configured clone was
created by VMAnyware or that its rendered plan matches the executed network.
No reader ACL grant on 201 is needed: administrative lab operations and their
observation are separate from the existing scoped read-only product token.

## Staged execution after approval

1. Verify identities, unused target/storage/bridge/port/routes. Generate exactly
   one disposable SSH identity in a new ignored directory, without overwriting
   files or accessing a personal agent. Record its public fingerprint; copy
   only the public key to the owned Proxmox node through the trusted console.
2. Create the runtime bridge. Do not reload the management network or edit
   persistent host networking configuration.

```sh
ip link add name vmbr1 type bridge
ip address add 192.0.2.1/30 dev vmbr1
ip link set dev vmbr1 up
```

3. Full-clone 9000 to 201 using verified HTTPS/API or the owned node CLI. Record
   the provider task reference, wait for explicit success with a deadline, then
   reobserve the new configuration before applying the smoke settings. An
   ambiguous timeout is not permission to replay clone.

```sh
qm clone 9000 201 --name vmanyware-debian-smoke-201 --full 1 --storage local-lvm
qm set 201 --cores 2 --memory 2048 --net0 virtio,bridge=vmbr1 \
  --ciuser debian --citype nocloud --ciupgrade 0 --ipconfig0 ip=192.0.2.2/30
qm disk resize 201 scsi0 12G
qm set 201 --sshkeys /root/vmanyware-smoke-201.pub --freeze 1
qm config 201 --current 1
qm cloudinit dump 201 network
```

The public-key file is newly generated for this smoke; no private key belongs
on Proxmox. Inspect generated user/network data locally without publishing
identity-bearing payloads. Require no password, extra NIC/disk or default route.

4. Start the owned bounded SSH relay and outer loopback host forward. Use the
   QEMU netdev identity observed by info usernet. Require both the Proxmox relay
   bind and host loopback bind to match the table. Do not bind wildcard addresses.
5. Start only 201 with freeze still enabled. Confirm CPU paused, attach an owned
   serial recorder to the observed socket, then continue CPU through the owned
   monitor. Keep serial output bounded and private until reviewed. Require the
   guest's generated Ed25519 host-key fingerprint in this trusted console before
   accepting any SSH key. If no trustworthy fingerprint is emitted, stop rather
   than accept the network key on first use alone.
6. Compare the SSH endpoint's observed Ed25519 key with that console fingerprint.
   Create a fresh endpoint-scoped known-hosts file only after an exact match.
   Use strict host checking, only the generated client identity and isolated SSH
   configuration. No existing key/config/agent, fallback identity, password or
   host-verification bypass is allowed.
7. Over the verified SSH connection, require Cloud-Init completion without error,
   Debian 13, user debian, two CPUs, roughly 2 GiB RAM, a 12 GiB virtual disk and
   expanded root filesystem. Keep package installation/snapshots out of scope.
   Capture command exit statuses, actual stdout and timings; do not equate
   provider task success or an open SSH port with readiness.
8. Exercise a real PTY, resize, foreground Ctrl-C and a fresh SSH reconnect if
   the host tools support reliable automation. Distinguish those OS OpenSSH
   results from the macOS Tauri terminal and document unexecuted checks.
9. Gracefully shut down only the observed clone and confirm stopped state.
   Stop owned relay/recorder and remove only the added outer forward. Remove
   vmbr1 only after 201 is stopped and no interface remains attached. If the
   guest will not shut down, retain its network/observations and report the
   failure; do not force-stop or delete it automatically.

## Acceptance boundary

A passing run proves nested guest execution and manual Cloud-Init/verified SSH
on this disposable topology. It does not complete the API-driven application
provisioning roadmap, validate DHCP, prove mobile SSH or test snapshots/restores.
The full clone and its definition remain available for a later separately
approved snapshot/lifecycle slice; template 9000 remains unchanged and stopped.

## Execution evidence — 2026-10-18

The user approved this concrete plan. Rechecked unused VMID/interfaces/routes/
ports/storage before mutation, then created runtime vmbr1 and one new local
disposable Ed25519 identity (private file mode 0600). No personal agent, config
or credential was used. Public key was supplied directly through the verified
HTTPS configuration API rather than copied into the example node-side file.

Full clone, resize, frozen start and graceful shutdown used the Proxmox API
with bounded polling of recorded UPIDs and explicit exitstatus OK. These were
administrative lab operations; no product mutation method or reader privilege
was added. Configuration readback required 2 cores/2048 MiB, scsi0=12G, one
private NIC/static address, generated public key, no guest password and frozen
startup. Template 9000 stayed stopped.

QEMU reported prelaunch with zero guest disk reads before the serial recorder
connected to the observed socket. After continuing execution, the captured
console reported Debian 13 and Cloud-Init final completion at about 6.8 seconds.
The bounded Python transport listened only at 10.0.2.15:18222, and the outer
forward was verified at 127.0.0.1:18222. No DHCP/NAT or management network
configuration was changed.

The captured public host-key block contained the generated Ed25519 key. Its
fingerprint was independently matched against the SSH endpoint before writing
the isolated known-hosts file:
`SHA256:zvz6i2RN7GQnorLwVp1Arw8/aAQ0rIbEC4/2bVwCGRY`.
Strict SSH used only the generated client key and explicitly disabled agent/
identity fallback. A deliberately mismatched local known-hosts file was rejected
(exit 255); no guest host key was changed to perform this negative check.

Actual guest observations over authenticated SSH:

| Observation | Result |
| --- | --- |
| OS/user | Debian GNU/Linux 13 (trixie), debian |
| CPU/RAM | 2 CPUs, 1974 MiB MemTotal |
| Disk | 12,884,901,888 bytes (12 GiB) |
| Root filesystem | 12,468,432,896 bytes; expanded from the image size |
| IPv4 route | Only 192.0.2.0/30 via eth0; no default route |
| Reconnect | Fresh verified OpenSSH connection succeeded |
| PTY resize | stty 24×80 then 43×132 |
| Foreground Ctrl-C | sleep interrupted; status 130 observed in 0.022 seconds |
| Session cleanup | Shell remained usable, SSH child reaped; no remaining debian sleep process |

An initial PTY assertion expected a printf after sleep in the same interactive
command list. Bash cancels that remainder on Ctrl-C. The harness now queries
status in a separate command after the prompt; the real interruption test
passes. This was a harness assumption, not an application change.

### Cloud-Init readiness limitation

The initial strict readiness command correctly failed because `cloud-init
status` returned **2**, not zero. Diagnostic JSON shows status=done,
extended_status=degraded done, no fatal errors, and recoverable DEPRECATED
messages: scalar `user` is deprecated in favor of a `users` list. Verified-HTTPS
readback of Proxmox's generated user-data confirms `user: debian`.
Cloud-Init 25.1.4 is the version observed in the guest. This warning has not
been suppressed or fixed, and no clean-readiness claim is made. The separate
resource/SSH checks above passed after recording the degraded result.

Upstream documents [exit code 2 as recoverable errors](https://docs.cloud-init.io/en/latest/explanation/return_codes.html).
Investigate the generated-user schema and select a correction/readiness policy
before reporting this recipe ready. A successful SSH login is insufficient to
turn the degraded status into a clean success.

### Preserved artifacts and cleanup

Graceful shutdown completed with provider task OK and stopped status. The owned
relay/serial recorder exited; the unused runtime bridge and outer 18222 forward
were removed. The temporary loopback script/serial receiver stopped, and the
Proxmox administrative console was logged out. Host checks show no listener on
18222 or 18473; the preexisting management/VNC endpoints remain.

VM201 and its 12 GiB disk are preserved, stopped, with freeze=1 and a NIC
referencing the removed runtime vmbr1. **Do not restart it without deliberately
restoring the approved network/capture setup.** Template 9000 is still stopped
and unbooted. Generated identity, serial output, task references and diagnostic
logs remain only in ignored `.lab/pve/clone201/`; they are not repository assets.
No guest, disk, token, ACL or snapshot was deleted or renewed.

This establishes nested guest execution and real Linux OpenSSH PTY feasibility.
It does not prove the macOS Tauri terminal against this guest, DHCP, clean
Cloud-Init readiness, application provisioning, snapshots or restores.

The next [compatibility candidate](cloud-init-compatibility.md) replaces only
the deprecated scalar user representation. Its actual 25.1.4 normalization/
schema checks pass offline. Fresh clone 202 and a custom user-data snippet are
proposed for a first-boot retest; no such mutation has been executed.

The subsequent [fresh-202 retest](cloud-init-compatibility.md) closes the clean
first-boot readiness gap for the pinned mapped-user recipe. It does not change
201's degraded first-boot result or implement product provisioning.
