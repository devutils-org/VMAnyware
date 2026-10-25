# 0093 — Explicit SSH host-key replacement

Status: accepted (2026-10-25)

## Context

Host trust pins one public key per endpoint and refused every changed key with
no recovery. The real home acceptance journey deleted VM 9001 and recreated
the retained definition as VM 9002, which received the same DHCP address
10.10.10.205 and a new host key. The application then had no way to reach the
recreated guest, although recreation always produces a new host key.

## Decision

- A changed-key inspection still refuses the connection and never authenticates.
  It now also returns the previously trusted fingerprint and a challenge of a
  distinct kind, bound to the window owner, endpoint and both observed keys,
  with the same two-minute expiry as first-use confirmation.
- A new IPC command `replace_ssh_host` takes the challenge, the previous
  fingerprint and the new fingerprint. First-use `confirm_ssh_host` refuses a
  replacement challenge without consuming it, and replacement refuses a
  first-use challenge.
- Storage replaces the pinned key only while the reviewed previous key is still
  pinned (under the existing lock), so a replacement reviewed against an older
  key cannot overwrite a newer pin.
- The UI shows both fingerprints, warns that a change can mean interception,
  and requires an explicit independent-verification checkbox.

## Consequences

Replacement remains a deliberate user decision after independent verification
(for example through the Proxmox console or guest agent). There is still no
automatic rotation, provider-attested key, or trust reset.
