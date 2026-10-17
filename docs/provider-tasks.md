# Read-only Proxmox task observation

The Rust provider client can observe an existing task with
`GET /api2/json/nodes/{node}/tasks/{upid}/status`. No HTTP passthrough route or
mutation method is exposed to the VMAnyware client. This helper is preparation
for a future worker executing approved operations; it does not create jobs,
persist provider references or reconcile tasks after a restart yet.

Construct `proxmox::TaskReference::new(node, upid)` from the intended node and
provider-returned UPID. Invalid/oversized references, mismatched embedded node,
path/query injection and malformed UPID fields are refused before traffic.
The bounded format accepts eight-digit PID/start-time hex fields and eight or
nine-digit process-start values supported by Proxmox. The task's Debug output
is redacted; do not log the raw identifier, which includes identity information.

`Proxmox::task_status` validates the response node and UPID against the reference:

| Provider response | Application result |
| --- | --- |
| `running`, no exit status | `TaskStatus::Running` |
| `stopped`, exact `OK` | `TaskStatus::Succeeded` |
| `stopped`, another nonempty exit status | `TaskStatus::Failed`, raw text discarded |
| Missing/blank stopped exit status, inconsistent running status, unknown state or identity | `Error::InvalidResponse` |

Warnings are conservatively treated as failure rather than clean success.
Existing fixed transport/authentication/TLS errors remain in effect. Raw provider
exit messages are not safe to display or log and are never part of this result.

`TaskPollOptions::new(interval, timeout)` requires intervals from 100 ms to
60 seconds and an overall timeout at least the interval and at most one hour.
`wait_for_task` performs an immediate read, sleeps between running observations,
and stops on a terminal state, first provider error or overall deadline. Dropping
the future cancels local observation; it never cancels the provider task. A
timeout also leaves the provider outcome unknown, not failed or rolled back.
No automatic mutation retry is implied.

A successful clone/start task is not proof of Cloud-Init completion, DHCP,
SSH access or guest readiness. Those need separately implemented checks and a
real disposable VM test. The helper has been checked using localhost TLS
fixtures only; no real Proxmox task has been observed.

The official API allows the task owner or an identity with `Sys.Audit` on the
node to read status. Verify the effective user/token privileges on the actual
host; this is a documented read requirement, not a tested provisioning role.

References: [Proxmox API schema](https://pve.proxmox.com/pve-docs/api-viewer/apidoc.js),
[Proxmox UPID implementation](https://github.com/proxmox/pve-common/blob/master/src/PVE/UPID.pm).
