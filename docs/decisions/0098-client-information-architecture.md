# 0098 — Client information architecture: Keychain, Console, Sessions, Activity

Status: accepted (2026-10-25), refines ADR 0094

## Context

After the first shell (ADR 0094) the same concepts still lived in several
places: the application identity panel appeared in creation, settings and the
local workspace; "Connect" opened a host form without the machine's address;
the terminal rendered under two nested forms; history was split into six
per-kind panels requiring operation IDs. Comparable products (DigitalOcean,
Hetzner Cloud, Termius, OrbStack, Proxmox) keep keys in one place and
reference them, give the terminal its own surface, and attach the address to
the machine.

## Decision

- Navigation: Environments · New · Sessions · Activity · Keychain · Settings.
  A machine has Overview · Console · Snapshots · Activity tabs.
- **Keychain** owns the application identity (create, unlock, system key
  store) and lists trusted host keys, with forget requiring the exact pinned
  fingerprint (new native `list_ssh_hosts`/`forget_ssh_host`). Creation only
  references the existing key and never offers to create one.
- **Console** (per machine, or ad hoc in Sessions) pre-fills address and user
  from `lastGuest` (ADR 0096), checks the host key and asks for review only
  when it is unknown or changed, then opens the identity (system key store
  when remembered, else passphrase, or SSH agent) and starts a session.
- **Sessions** own terminal lifetime as tabs that stay mounted across
  navigation; closing a tab closes the native session.
- **Activity** is one read-only timeline (global, or filtered on a machine)
  merging per-kind histories; opening an entry shows the existing validated
  saved-operation detail focused on that ID.

## Consequences

Host trust, identity consent and recovery validation are unchanged; only
where they are presented moved. The former SshHostTrust/SshAuthentication
views remain for the local developer workspace.
