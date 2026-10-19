# ADR 0031 — Durable Cloud-Init API-settings stage

Date: 2026-10-18
Status: Storage and standalone HTTPS executor tested locally; production caller pending.

Migration 9 adds a private per-intent Cloud-Init dispatch record containing the
exact configuration revision, primary volume and observed original net0 string.
Original definition/catalog/plan inputs already bind user, bridge and public keys;
they are not duplicated into an executable payload. The record is committed with
the stage dispatch marker and clears the operation's current task reference.
Earlier clone/resize task references remain on their respective stages. This
synchronous configuration API has no task of its own.

Claim only configure_cloud_init after all earlier plan stages succeeded. Revalidate
the full clone/resource evidence chain and optional disk completion, including
scope/instance/plan, task success, requested resources/disk, volume and stopped
power/revision. If no expansion was planned, use resource completion's revision;
otherwise use disk completion's revision and exact retained volume. This handles
the stage's conditional position without creating a fake disk success.

Before dispatch, validate the original full-plan approval and construct the fixed
Cloud-Init request from retained definition and observed volume/network/revision.
The revision and optional expanded volume must match prior evidence. Readers
validate dispatch/stage consistency and reconstructed inputs; invalid prior or
retained state makes a claim roll back atomically. The first clone approval window
is not renewed; later-stage approval lifetime follows ADR 0027.

Undispatched queued/interrupted work may be reclaimed. A dispatched uncertain
stage can only perform future GET reconciliation, including without a task ID;
begin rejects a second dispatch. Reconciliation has the existing five-second
backoff. Before dispatch, definite capability/catalog/provider/target failures may
fail the stage. Once marked, errors remain uncertain and cannot justify retrying
PUT. Failure retains the local target association and blocks later/lifecycle work.

Completion binds instance/scope/plan, exact volume/requested disk bytes,
canonical target network, catalog guest user, key count and observed current
resources/stopped target/revision. Require guestReady=false: this evidence is API
settings only. Persist result on configure_cloud_init; whole workflow stays running
with no result and all subsequent stages remain queued. A completed settings stage
cannot be claimed again. Seed contents, regeneration, compatibility, start and
strict readiness remain separate unfinished requirements, including ADR 0017.

Five new SQLite tests pass: both expanded/unexpanded paths and result binding,
pre/post-marker restart/backoff and no redispatch, schema-8 upgrade with retained
disk evidence, corrupted prior/dispatch state with claim rollback, and failed
preconditions retaining the target. Nineteen affected prior journal cases and two
older migration cases pass in targeted runs. Scoped library/intent-test Clippy
passes after boxing the larger success outcome; its persisted JSON is unchanged.
No provider execution, schema dependency, credential or infrastructure mutation is
part of this slice; no first-boot or readiness gate is closed.


The standalone executor now observes the mapped disk volume/net0, verifies the
retained source revision and optional expanded volume, and repeats the fixed
stopped-target/resource/disk/Cloud-Init-drive checks before committing dispatch.
It requires current capability/catalog/scope for new writes. After the marker,
transport repeats those checks and sends at most one synchronous PUT. Independent
settings observation follows even after a malformed or lost response; unproven
outcomes stay uncertain. Restart uses retained inputs and GET only, even with
creation disabled and no current catalog. No production worker/API invokes it.

Three targeted HTTPS executor tests pass: one PUT across normal/lost/malformed
reply and applied/unconfirmed results, GET-only disabled restart, and pre-marker
capability/catalog/scope/volume/revision/network/drive gates. The gate fixture was
corrected to route reads by path and that test rerun independently. Three affected
transport tests pass after extracting the shared read-only guard; scoped library
Clippy passes. No broad storage/client/native suite was repeated. This still proves
API settings only, not generated seed compatibility or strict first boot.
