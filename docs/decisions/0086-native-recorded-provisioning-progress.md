# 0086 — Native recorded provisioning progress

Status: implemented; current acceptance evidence is recorded in STATE.md.

## Decision

One fixed `read_remote_provisioning_progress` command reads an existing operation,
its retained environment and its public provisioning progress through authenticated,
verified HTTPS GETs. Existing strict creation recovery validates the association
without installing a prepared plan, request key or execution consent. The progress
projection must match operation/instance, definition hash, provider scope, node/VMID,
status and failure. Successful readiness must also match the plan hash. Optional
source identity must be canonical and distinct from the new instance.

Only known stage kinds/statuses and provisioning failure codes are decoded. The
five required stages and optional disk/packages/final-snapshot stages must occur
in exact sequential order with consecutive positions. No stage can have started
past an incomplete predecessor. Contradictory completion and unknown stage kinds
are refused. Typed serialization discards private extra fields, stage results and
raw provider errors. A worker transition between independent GETs can produce a
fixed mismatch error; the user refreshes the read, never the mutation.

The command is explicitly registered in the production handler, reviewed command
manifest and local-main capability. The surface now has 77 commands. No generic
HTTP bridge, remote-window grant or new runtime dependency is introduced.

The four React creation/recreation/recovery views share an explicit read/refresh
control and ordered stage list. Known plan/source/target values are checked again
before rendering. Replacement, disconnection or editing the parent clears/discards
old results. Read errors use a fixed message, including unsupported older servers.
The view labels its data as saved evidence; dispatch is not completion, live power,
guest readiness or SSH reachability. It never grants SSH, changes operation status,
submits execution or restores consent. Existing status/guest-proof controls keep
those responsibilities.

## Validation boundaries

Native protocol checks and real React/Firefox simulated-IPC scenarios are separate
from the joined Linux WebView/native/API/generated-provider journey. Neither proves
real Proxmox/guest behavior or execution of the current Android APK. Artifact
refresh and actual platform acceptance retain their separate gates.
