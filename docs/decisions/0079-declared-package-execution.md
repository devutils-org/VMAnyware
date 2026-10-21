# 0079 — Bounded declared Debian package execution

Status: provider transport and durable worker implemented/tested; final snapshot worker now implemented under ADR 0080; API admission now passes under ADR 0081; native client integration pending.

## Decision

Declared package installation uses the existing verified Proxmox QEMU guest-agent
transport, bound to the retained clone identity, plan marker, configuration digest,
running power/QMP state and absence of pending configuration. It does not require
storing a guest private key on the server. This supports Debian templates with
apt-get, dpkg-query, coreutils timeout, /bin/sh and the guest agent installed.
At the initial transport slice, admission remained closed until the durable stage
was integrated. ADR 0081 records the subsequent API activation; native integration
and actual guest acceptance remain open.

The command is fixed, has a 900-second execution timeout with a 30-second kill
interval, bounded package-lock wait and noninteractive environment. It updates
indexes, installs only declared packages/dependencies without removals and checks
literal installed status and requested versions with dpkg-query. Declarations are
separate positional arguments, never interpolated into shell source. Request
construction rejects options, shell syntax, package removal suffix, release/glob
selectors, duplicates, empty or excessive input. Literal names ending in plus such
as g++ remain supported and verified. The maximum is 128 declarations of 128 bytes.
Repositories and package maintainer scripts remain part of the operator-approved
template trust boundary. This does not pin unversioned dependencies or indexes.

APT supports action suffixes and version selection; see the
[Debian apt-get manual](https://manpages.debian.org/bookworm/apt/apt-get.8).
Consequently argument separation alone is insufficient: --no-remove, declaration
validation and post-install literal-name/version checks are all required.

Raw stdout/stderr are redirected in the guest and never returned to operation
status or diagnostics. Only pending/completed/failed evidence escapes. Contradictory
process metadata, truncated/nonempty output and malformed PID are rejected. Status
reads bind the exact retained PID and verify target identity before and after.
There is no transport retry, automatic cancellation or automatic rollback.

## Required durable integration

Before dispatch, persist the one-time mutation marker and a retained stage deadline.
Persist the returned PID before any status read. A lost reply or restart between
marker and PID must remain uncertain, never redispatch. A known PID may be observed
without issuing another mutation. Only zero exit with verified installed packages
may advance to the optional final snapshot; failure cannot mark provisioning ready.
Retain the original definition/plan/instance scope, honor creation permission before
first dispatch and prevent snapshot dispatch when its independent write gate is off.

## Evidence

Focused generated-HTTPS tests exercise fixed POST/body, scoped GET/PID, pending/
completed/nonzero/signal results, malformed/leaking responses, changed digest before
POST and exactly one POST after a lost reply. Shell syntax is checked with sh -n;
no package command runs on the developer host. Worker/restart/API and actual guest
installation evidence are explicitly absent at this stage.

## Durable worker acceptance

The sequential production provisioning queue now claims the package stage after
all predecessors succeed. A backward-compatible optional packages member of the
private seed journal retains startedAt, a fixed 960-second deadline and the PID.
The existing stage dispatch_started bit is committed before the single POST. The
worker checks the retained approval against the current catalog and the independent
creation gate before first submission. Known PID recovery needs neither the current
catalog nor enabled writes, but still requires the same provider scope and strict
target guard. Unknown PID recovery does not contact the provider or redispatch.

Each round observes at most once and retains its next read time. A missing reply
waits for the retained deadline. Expiration leaves the outcome uncertain and stops
automatic polling, since the guest process may still have been running. A known
nonzero/signal result fails the stage and retains the VM. Only confirmed zero exit
can produce a scoped installed/packageCount proof. Provisioning succeeds only if
this is the last declared stage; a following snapshot remains queued. Existing
readiness evidence is retained in the final operation result for this stage-only
workflow. Invalid PID rebinding, deadline corruption and malformed completion proof
are rejected when reading or updating SQLite.

Six focused generated-HTTPS/SQLite tests pass: pending-to-success with database
reopen and writes disabled, lost-reply no replay through restart/deadline, disabled
first dispatch/nonzero failure, forged completion/PID rebinding, exact process death
after dispatch marker, and linked-PID recovery/corruption refusal. The two affected
durable readiness tests and seed-regeneration restart test also pass. This does not
open API admission or prove actual guest package installation. Final-snapshot and
client result validation must be integrated before removing the existing rejection.

The optional final snapshot worker now also has separate generated-HTTPS/SQLite
acceptance (ADR 0080). API/client admission and full declared-workflow integration
still precede removing the existing unsupported-feature rejection.
