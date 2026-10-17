# Cloud-Init scalar-user compatibility

Status: fresh clone 202 first boot verified on 2026-10-18 with Cloud-Init exit
0, done and no fatal/recoverable errors. The original degraded first boot on
201 remains recorded in [the clone smoke](x86-clone.md).

## Cause and minimal correction

The generated Proxmox user-data has a scalar `user: debian` and a default-user
list. Cloud-Init 25.1.4 converts the scalar into a name dictionary internally
but records a recoverable deprecation, resulting in status exit code 2.

The candidate changes only this field's representation:

```yaml
# Before
user: debian
# Candidate, in place of the scalar
user:
  name: debian
```

The existing `users: [default]`, hostname, SSH public keys and other allowed
passwordless settings are retained. This preserves distribution default-user
attributes instead of constructing a new user with guessed groups/sudo rules.
It does not suppress warnings or replace exit-code handling with unconditional
success. This is scoped to the tested version; upstream may change the contract.

The [offline experiment](../experiments/cloud-init-user/README.md) includes a
strict converter and semantic/schema checks using actual upstream Cloud-Init
25.1.4 source. Both its synthetic case and the stopped 201 clone's real
user-data pass: equal normalized users/groups, unchanged selected public key,
valid schema, one original scalar deprecation and zero converted deprecations.
This does not execute a guest's Cloud-Init modules or prove first-boot exit zero.

No product dependency was added. Upstream source/requirements were downloaded
only into an ignored experimental venv. No Proxmox package patch, guest package
change, new token or reset of the old instance cache is part of this preparation.

## Approved fresh first-boot retest

A reboot of 201 is not a clean retest: its once-per-instance results are already
cached. Preserve 201 as stopped diagnostic evidence and leave template 9000
unchanged. The user approved these concrete operations on only disposable pve-lab:

| Resource | Approved operation |
| --- | --- |
| Clone | Full clone template 9000 to unused 202, vmanyware-cloudinit-compat-202 |
| Resources | 2 CPUs, 2048 MiB, 12 GiB scsi0 on local-lvm |
| SSH identity | New disposable Ed25519 key for 202 under ignored .lab/pve/clone202; no password or personal agent |
| User-data | Dump 202's generated data after its own public key/user/hostname are set; convert that fresh document, not candidate201.yaml |
| Snippet capability | Add snippets to storage local's existing import/backup/iso/vztmpl content list; preserve all other settings |
| Snippet file | New /var/lib/vz/snippets/vmanyware-202-user.yaml, mode 0600; refuse an existing file |
| Binding | Set only 202's cicustom user=local:snippets/vmanyware-202-user.yaml; retain generated network and metadata |
| Network | Recreate the prior runtime-only vmbr1/private /30 and loopback 18222 relay/forward, exactly as in the prior smoke |
| Boot/trust | Frozen startup, serial capture before CPU continuation, independent guest host-key comparison before strict SSH |
| Completion | Check first-boot Cloud-Init exit 0, done, no fatal/recoverable errors, intended user/key/resources and expanded root filesystem |
| Cleanup | Gracefully shut down only 202; remove owned relay/recorder/forward/bridge; preserve stopped clone, disks, snippet and approved storage capability |

Read-only observations at preparation time: 202 available, both 201/9000 stopped,
local-lvm has 19,140,628,710 bytes free, and local's content set is exactly
import/backup/iso/vztmpl. Recheck immediately before execution and refuse
collisions/changed configuration. The snapshot/restore/delete operations,
existing API token expiry, reader ACLs, management network and Linux host
networking remain outside this proposal.

A custom user-data snippet replaces generated user-data entirely. It must be
regenerated and rebound when its hostname/key/user inputs change. Simply
changing ciuser/sshkeys later does not merge new values into a static snippet.
Verify the file content and source binding before first boot; never reuse the
201 candidate for 202 or pass an arbitrary user-supplied script as a workaround.
No application provisioning integration is implied by this manual lab recipe.

## Evidence required before accepting the workaround

Require provider task completion separately from SSH/readiness. Store the actual
first-boot Cloud-Init JSON/exit status locally, plus resource/key/host-trust
observations. If another warning occurs, record it and keep readiness open;
passing normalization/schema checks does not authorize ignoring new warnings.
No cleaning/rerunning Cloud-Init on an existing guest is proposed. Do not patch
installed provider code to make this experiment pass.

Sources: [Proxmox generator](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuServer/Cloudinit.pm),
[exact Cloud-Init normalizer](https://github.com/canonical/cloud-init/blob/25.1.4/cloudinit/distros/ug_util.py),
[configuration reference](https://docs.cloud-init.io/en/latest/reference/modules.html#users-and-groups),
[return-code semantics](https://docs.cloud-init.io/en/latest/explanation/return_codes.html).

## Actual fresh-202 evidence — 2026-10-18

Full-clone, resize, frozen-start and graceful-shutdown provider tasks completed
with OK. Only clone 202 used the corrected custom user-data. The local storage
content capability gained snippets while its other settings were preserved;
the new snippet was created exclusively with mode 0600. Template 9000 and the
original diagnostic clone 201 remained stopped and unchanged.

First-boot Cloud-Init 25.1.4 returned **0**, status/extended_status **done**,
errors `[]` and recoverable_errors `{}` globally and in every stage. Final-stage
completion was 6.95 seconds after boot. The guest's consumed
`/var/lib/cloud/instance/user-data.txt` SHA256 matched the prepared snippet:
`860234cd4538133847e1737c8e4613a87de4e14d58fc3da0dc4c277b7deadc35`.
No cache reset, warning suppression or provider/guest package patch was used.

Independent first-boot serial Ed25519 key capture matched the SSH endpoint
before strict trust was stored. Selected-key authentication and fresh reconnect
passed; a different pinned host key was rejected with exit 255. Actual Debian
13 user debian had 2 CPUs, 1974 MiB RAM, a 12,884,901,888-byte disk and
12,468,432,896-byte root filesystem; only the private /30 route, no default
route. OpenSSH PTY resized 24x80 to 43x132; Ctrl-C interrupted foreground sleep
with status 130 in 0.036 seconds, leaving a usable shell and no sleep process.

This proves the pinned lab recipe with administrative API access and Linux
OpenSSH. Product provisioning methods, least-privilege mutation ACLs, DHCP and
native macOS access to this real guest remain unproven. Raw tasks, serial/status
logs and disposable identities stay under ignored `.lab/pve/clone202`.

Cleanup verified: 202 gracefully stopped; final API readback shows 202/201/9000
stopped. Owned relay/serial recorder, runtime vmbr1, added loopback 18222 forward
and host HTTP receiver were removed/stopped; root console logged out. Stopped
202, disks, snippet and approved storage capability remain. Its freeze=1 and
missing vmbr1 require deliberate restoration before another start. No snapshot,
restore or deletion was performed. See [ADR 0017](decisions/0017-cloud-init-user-compatibility.md).
