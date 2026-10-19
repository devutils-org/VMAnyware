# Read-only lifecycle preflight

`GET /api/v1/environments/{id}/lifecycle-preflight` checks an existing registered
association without changing it or its VM. It uses the saved provider scope,
node and VMID; query parameters cannot retarget the read. Missing instances,
configuration, mismatched scopes and provider failures use the same safe error
mapping as instance observation. Confirmed start/shutdown endpoints are documented separately in [the API](api.md).

The response binds `instanceId`, `providerScopeId`, `definitionHash` and an
`expectedMarker` (`vmanyware-<instance-id>`). It contains a sanitized power/QMP
`observation`, provider `configDigest`, and five fixed checks:

| Code | Required observation |
| --- | --- |
| `instance_marker` | Exactly one VMAnyware-prefixed tag, matching this instance |
| `not_template` | Target is not a template |
| `unlocked` | No provider lock is present |
| `no_pending_changes` | No pending configuration or deletion |
| `configuration_unchanged` | Config digest agrees before/after the reads |

The config is read with `current=1`, followed by pending configuration, power
status and a second current-config read. All four GETs share a 20-second deadline;
existing verified TLS, redirect refusal and response bounds apply. Malformed or
missing required config/pending metadata fails the read rather than passing a
check. Tag lists with empty entries or surrounding whitespace are rejected.
Raw description, tags, lock text, passwords and config/pending values are omitted.
The expected marker is derived locally, never copied from arbitrary provider text.

`startPreconditions` and `shutdownPreconditions` are passed/failed/unknown.
Start requires stopped power and absent/stopped QMP state in addition to common
checks. Shutdown requires running power with running QMP execution. Paused,
prelaunch and suspended states do not pass a shutdown check. Missing/unknown QMP
execution on a running VM reports unknown, not success. Any common failure blocks
both actions. Guest readiness and SSH are not checked.

`executionEnabled` reflects the explicitly configured server capability, defaulting
to false, independently of whether preconditions pass. It never confirms an action. The marker
is a public administrative declaration and may be copied or stale; its match is
not independent ownership proof. Digests do not provide an atomic reservation.
No marker is installed, no existing tag is repaired and no token/ACL is changed.
Future writes require fresh identity/ownership checks and explicit authorization
with durable intent/task linkage. See [ADR 0022](decisions/0022-read-only-lifecycle-preconditions.md).

The [upstream Proxmox QEMU API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
documents current config/digest, pending values/deletes and VM status under
`VM.Audit` on the target. Installed-lab ACLs and this preflight's real-host behavior
are not newly verified here. Local generated-certificate fixtures exercise the
exact GET sequence, allowed/blocked/unknown cases, config changes, malformed
metadata, scope rejection before network access, TLS and permission failures.

## Client display

**Check lifecycle for VM** explicitly loads a report for a selected saved
association. The renderer binds its instance, scope, definition hash, marker and
node/VMID before displaying exactly the five known checks. It rejects malformed or incomplete responses and requires a boolean capability. Check labels are owned by the client;
raw response messages are not rendered. It displays enabled/disabled service capability; passing checks expose no mutation button.
Starting another read hides the old report, and changing the local service port
invalidates pending results/errors.

The disposable Linux Firefox simulated-IPC walkthrough covers explicit loading,
visible blockers, absent mutation controls/raw messages and port-change stale
report rejection. Two additional Node tests cover report identity/schema and
unknown/disabled states. Tauri GET command contract tests compile and pass on Linux with the temporary
[signed-package prefix](native-linux-prefix.md). No native GUI/WebView launch is
implied by those HTTP tests.
