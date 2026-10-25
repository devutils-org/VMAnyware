# 0097 — One reviewed-operation module for confirmed remote actions

Status: accepted (2026-10-25)

## Context

Start/shutdown, snapshot, restore, delete and recreate each carried their own
copy of the same state machine (review under a fresh request key, confirmed
submission, same-key retry, polling, release, lock reporting, stale-response
guards). Every UX change (auto-review, polling, automatic release) had to be
applied five times, with drift risk in safety-relevant code.

## Decision

`reviewedOperation.ts` owns the state machine behind a small interface:
`review(input)`, `submit()`, `refresh()`, `release()`, `cancel()`, `dispose()`.
Each action supplies an adapter: its native prepare/confirm/read/clear
commands, its review and evidence validation, its terminal definition and,
for lifecycle, the native refusals proven to precede any write. The React
binding `useReviewedOperation` adds automatic review, bounded polling,
automatic release of definitively settled requests and lock reporting.

Invariants kept in the module and covered by Node tests: one request key per
review; an unknown submission outcome keeps that key for retry; uncertain or
unsettled work is never released or abandoned; evidence that does not match
the review is reported, not accepted; disposed controllers ignore late
responses.

## Consequences

Action views keep only their adapter and rendering (about 70 lines each,
from about 110). Remote creation still uses its own flow because its plan
review and progress differ; it can adopt the module later.
