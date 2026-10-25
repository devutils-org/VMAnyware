# 0096 — Remember the last proven guest address as a connection hint

Status: accepted (2026-10-25)

## Context

The guest IPv4 and the image's guest user were only present in the result of
the creation or recreation operation. An environment carried no address, so
"Connect" from a machine started from an empty host field and users had to
find the IP in Proxmox.

## Decision

- Schema 17 adds `guest_address`, `guest_user` and `guest_observed_at` to
  instances. When guest readiness succeeds with its existing strict proof
  (cloud-init clean, routable IPv4, named interface), the same transaction
  records the address, the plan's guest user and the time on the registered
  instance.
- Environment summaries expose it as `lastGuest`. The native client accepts
  it only when the address is a non-loopback, non-broadcast IPv4 and the user
  is a plain identifier; otherwise the list is rejected.
- The client uses it to pre-fill a machine's Console. Host-key verification is
  unchanged: an address is a hint, never host identity, and a DHCP address can
  change.

Migrations 0014 and 0016 now copy instance columns by name, which is
equivalent for real histories and keeps rebuilt-history test fixtures valid.

## Consequences

Instances created before schema 17 have no address until a new readiness
proof. No on-demand guest-agent address read is added yet.
