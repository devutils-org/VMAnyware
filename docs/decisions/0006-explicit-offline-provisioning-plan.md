# ADR 0006 — Server-owned template mapping and a stateless offline plan

Date: 2026-10-16
Status: Implemented; live provider checks and execution remain pending.

Require the operator to map image identifiers to explicit template/node/version,
boot disk size/device, target storage/bridge and guest user. No VMID, version,
storage or bridge is inferred from an image name. Load this strict catalog only
from the explicitly configured `VMANYWARE_TEMPLATES` path, without credentials.

Build plans from current validated YAML, the selected mapping and an explicit
candidate VMID. Bind those inputs with deterministic hashes and refuse disk
shrinking/source-VMID reuse. Keep the first plan limited to same-node full clones
and IPv4 DHCP. Record all other live capability/availability checks as unverified.

Use a stateless endpoint instead of extending the worker or database: computing
this plan has no external side effect or long-running work. Return conceptual
steps and requested package/snapshot intent without implying an implementation
exists. No new runtime dependencies or database migration are needed.

Display the preview in the native app next to the current definition. Tie the
display to the exact YAML/port/VMID that produced it so an edited input cannot
leave a stale plan looking current. There is no execution/approval action in
this slice. Future execution must revalidate provider state and require a concrete
approved test target; the plan hash is not an authorization token.
