# 0083 — Explicit package and final-snapshot review

Status: React/browser and joined Linux native create/recreate integration pass; fresh delivery/platform/real-guest acceptance remains open.

## Decision

RemoteCreate now accepts optional Debian package declarations, one per line, and
an initially unchecked final disk-only snapshot option. Blank lines are discarded
and declaration order/pinned versions are retained. Existing server/native grammar
and bounds remain authoritative. The UI explains repository/dependency variability,
the independent snapshot permission and snapshot consistency/backup limits.
Public-key input remains distinct from all guest provisioning declarations.

Preparation sends these exact values in the saved definition. Before exposing a
review, the renderer rejects a returned plan whose package sequence or snapshot
boolean differs. Review shows the native retained plan, not current editable form
state. Fields lock through the existing fieldset while preparing/reviewing/submitted.
Edit definition removes the review and consent; re-preparation uses a fresh request
key. Once submission is uncertain, editing stays unavailable and explicit retry
uses the same retained native approval. The creation confirmation has a stable
create-confirm ID, separate from the new option checkbox.

Recreation displays exact declared packages and final snapshot intent next to the
retained YAML and fresh-disk review. Declared final snapshots are newly created
snapshots of the fresh instance; prior snapshots/disk contents are not restored.
Existing capability/preflight/consent and read-only status rules remain unchanged.
No option initiates SSH, authentication, installation or snapshotting by itself.

## Evidence

Frontend typecheck/build and the --declared-create real React/Firefox scenario
with simulated IPC pass. The scenario covers initially empty/unchecked options,
changed server intent refusal, exact pinned-package/snapshot review, fieldset lock,
editing/re-preparing with new key and unchecked consent, identical submission retry,
status/read-only recovery and independent SSH host trust. The test launcher verifies
that this scenario was actually active. Existing --remote-connection and --recreation
checks also pass; recreation asserts its retained feature summary.

The first read-only assertion incorrectly used the input disabled property for an
inherited fieldset lock; it now checks the effective :disabled state. Native Android
creation's test driver uses create-confirm too; its syntax passes, but it has not
been rerun on a device for this slice. This is browser evidence, not actual native
IPC/provider/guest/Android acceptance. No dependencies or VM operations were added.

## Joined native acceptance

Both --declared-create and --declared-recreation now pass actual React/WebKit,
Tauri IPC, verified HTTPS/authenticated API and sequential worker integration with
an owned generated provider. Declared packages, final snapshot, explicit consent,
lost accepted reply and identical-key retry are exercised from the UI. Provider
ordering checks reject packages before Cloud-Init evidence and a snapshot before
confirmed package completion. SQLite final proofs and exact submission counts are
checked independently; the fresh/retired source distinction remains intact.
The shared default recreation/app+server restart/read-only recovery scenario also
passes. Native example and current debug server compile before these runs.
No real guest executes apt; no real VM or current Android runtime is validated.
The next task is artifact delivery for this now-integrated source, then its
platform/approved real-target acceptance.
