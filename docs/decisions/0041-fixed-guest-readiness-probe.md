# 0041 — Fixed clean Cloud-Init guest probe

Status: fixed transport locally tested; durable readiness executor pending.

A successful start task and running VM do not prove guest initialization. Add
fixed guest-probe inputs bound to the plan/instance marker, expected configuration
revision and the configured NIC's MAC. Before commands and observations, verify
that marker/revision, non-template/unlocked state, running/QMP state and absence
of pending provider changes. Ready observations repeat this guard afterward.

The [upstream agent API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu/Agent.pm)
provides command-array execution, PID status and network-interface reads. These
exec endpoints currently require VM.GuestAgent.Unrestricted; a narrower effective
ACL is not claimed. [Provider agent code](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuServer/Agent.pm)
decodes stdout/stderr and normalizes booleans to integers. Handle both boolean
and 0/1 flags, reject other representations and never decode stdout twice.

Submit only /usr/bin/cloud-init status --format json as a JSON argument array.
No shell, stdin, executable/argument passthrough, package install, key/credential
change or wait flag. Retain a positive bounded PID; later observations use only
fixed GET exec-status?pid and network-get-interfaces. Each method has a 20-second
deadline. This is not an overall provisioning timeout; the executor must persist
its own deadline/PID and enforce the retained successful start gate.

The pinned Cloud-Init 25.1.4 source was inspected locally. Require process exit 0,
status/extended_status done, empty global and per-stage fatal/recoverable errors,
and all four stages with valid completed timestamps. Running/not-started or an
unfinished process is pending, never ready. Reject stderr, signals, truncated or
malformed output, exit 2, missing stages and degraded/disabled states. stdout is
bounded to 64 KiB and never included in safe results/errors.

Require exactly one usable IPv4 on exactly one MAC-matched interface. Ignore
other NICs, reject ambiguity, invalid prefixes and unsafe addresses; loopback,
link-local, subnet/broadcast and unspecified addresses do not count. Return only
IPv4/interface and clean-Cloud-Init/exit-code evidence. Provider network reports
do not prove SSH reachability, installed key authentication or trusted SSH host
identity. Supplied revision drift, including benign provider changes, fails closed
until explicitly reconciled; real post-start behavior remains untested.

Three scoped generated-HTTPS tests pass: exact command/PID/routes and both flag
encodings; pending/failed/truncated/error/address/revision/QMP cases; and foreign
markers, invalid PIDs and lost responses without command retry. Library Clippy
passes. No runtime dependency/schema change, production integration, real guest
command, credentials or infrastructure mutation occurred. Next implement durable
PID/deadline and readiness result, then connect the minimum create API/worker/UI.
