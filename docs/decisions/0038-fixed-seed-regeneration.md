# 0038 — Fixed synchronous seed regeneration

Status: transport tested locally; durable journal/start integration pending.

The [upstream QEMU API implementation](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
implements PUT cloudinit synchronously under a configuration lock and returns
null. It accepts node/VMID only, without a configuration-digest argument or task
identifier. Do not invent a task reference or send an unsupported digest.

Add a fixed transport using the existing validated CloudInitRequest. Before PUT,
repeat generated-seed preflight and match the caller's expected compatible report:
revision, user/network hashes and user format. Reject incompatible or guest-ready
reports. Submit an empty form to the exact target's cloudinit path once. Only a
successful JSON null response permits postflight. Repeat settings/seed inspection
and require unchanged compatible payload hashes and user format. Return only the
safe postflight report, with guestReady=false. Total deadline is 45 seconds.

The configuration digest may change when regeneration updates provider metadata;
postflight still checks its own stable revision. Since the endpoint offers no
compare-and-swap, checks cannot exclude an intervening operator edit followed by
restoration. They are bounded observations, not atomic ownership or ISO inspection.

The caller must commit a dedicated dispatch marker before this method. Lost,
malformed or timed-out replies are uncertain and never authorize replay; generated
preview GETs alone cannot establish that regeneration occurred. Production callers
must retain successful evidence before starting. This transport is not yet called
by the production worker or existing settings-only executor. Add that journal and
start gate next; never reinterpret old settings results as regeneration evidence.

One generated-HTTPS test covers success, incompatible/revision/hash mismatch
before PUT, lost response, unexpected task-like reply and changed postflight
contents. It asserts the exact empty authenticated PUT and at most one submission.
No real provider access or VM mutation occurred.
