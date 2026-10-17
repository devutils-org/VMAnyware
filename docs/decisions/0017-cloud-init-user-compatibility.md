# 0017 — Pinned Cloud-Init user compatibility and strict readiness

Date: 2026-10-18
Status: accepted for the disposable lab recipe

## Context

Proxmox VE 9.2.2 generates scalar `user: debian`. Debian 13's tested
Cloud-Init 25.1.4 accepts it with a recoverable deprecation and exit 2.
Clone 201 demonstrated that successful provider tasks and SSH alone do not
prove clean guest initialization.

## Decision

For this pinned passwordless/default-user recipe, convert only the scalar user
into `user: {name: debian}` and preserve the generated remaining configuration.
The offline converter refuses unrecognized shapes. Bind a new target-specific
custom user-data snippet; preserve generated network and metadata. Regenerate
the complete snippet whenever hostname/user/key inputs change.

Keep readiness strict: require first-boot Cloud-Init exit 0, done and no fatal
or recoverable errors, independently of provider task success. Do not suppress
warnings, patch installed packages or reset instance cache to manufacture proof.

## Evidence and consequences

[Fresh clone 202](../cloud-init-compatibility.md) passed actual first-boot
checks, consumed snippet hash, selected-key strict SSH, resource expansion,
PTY and reconnect. Original 201 remains stopped diagnostic evidence.

This requires snippets-capable storage and administrative lab operations.
It does not establish least-privilege execution ACLs or implement application
provisioning. Keep the converter outside product dependencies until an actual
execution use case defines ownership, input changes and snippet lifecycle.
Future versions require renewed compatibility evidence; unrelated warnings
remain failures under this recipe's readiness acceptance criteria.
