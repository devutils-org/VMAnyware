# ADR 0002 — Prepare the local API while native validation is blocked

Date: 2026-10-16
Status: Accepted implementation scope; runtime validation pending.

## Context

The user requested implementation after the foundation was prepared. Rust and
Cargo remain unavailable, and an actual npm install failed with registry DNS
resolution errors. Repeating the same blocked setup work would not advance the
implementation. Phase 0 allows a minimal testable seam when infrastructure is
unavailable; the native baseline remains unproven.

## Decision

Prepare a small Axum service with a real health route and in-process router
tests. It is the eventual host for the Proxmox spike, without adding provider,
database or orchestration layers yet. This is a narrow exception to the initial
plan to defer all service source until native validation passes. It does not
complete Phase 0 or authorize starting full MVP implementation.

Runtime dependencies are Axum for HTTP/JSON routing, Tokio for asynchronous
serving and shutdown signals, and Serde for typed response serialization.
Development dependencies are Tower's service utility for exercising the router
and serde_json for checking complete response contracts.

Expose only process liveness and bind strictly to IPv4 loopback until explicit
network authentication is implemented. Accept a validated port override, not a
bind-address override. Keep errors fixed and independent of request/configuration
contents. Native UI integration waits until these builds can be exercised.

## Consequences

The server source can be checked independently with `make check-server` on an
equipped machine. It still shares the workspace dependency lock. All source
and tests remain uncompiled here; no successful HTTP response is claimed.
