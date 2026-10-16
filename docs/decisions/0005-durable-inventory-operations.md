# ADR 0005 — Prove durable operations with read-only inventory

Date: 2026-10-16
Status: Implemented for inventory only; VM task orchestration remains pending.

Use an existing real workflow, inventory refresh, to prove operation durability
before implementing Proxmox mutations. Store queued/running/terminal records in
SQLite, return HTTP 202 after commit, and let the native client poll by ID. This
keeps execution progress distinct from provider/guest readiness.

SQLite generates opaque operation IDs, deduplicates request keys, enforces one
active refresh and atomically claims jobs. No new runtime dependency is needed.
The binary owns a single worker. Database failure in the worker stops the
service; startup preserves uncertainty by marking running jobs interrupted.
It never pretends an unconfirmed action succeeded.

Use Rust's standard File locking to prevent two services recovering/working on
the same local database at once. Hold the lock before migration and keep the
handle alive until shutdown. Do not unlink lock files or treat their existence
as ownership. This is single-instance local storage, not a distributed lease.

Idempotency currently has one fixed read-only input. Future clone/start/restore
operations must bind their request keys to target/configuration hashes and
persist provider task references before they can be safely reconciled. Do not
extend this worker with blind retries of ambiguous VM mutations.
