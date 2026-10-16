# ADR 0007 — Read-only observations before provisioning

Date: 2026-10-16
Status: Implemented and verified with local TLS fixtures; real-host validation pending.

Keep the offline plan deterministic and add a separate stateless preflight
endpoint. Recompute the plan from validated YAML and the server-owned catalog,
then perform bounded GET requests through the existing verified-TLS client.
No mutation methods, database schema changes or new dependencies are needed.

Report each observation as passed, failed or unknown. Permission-filtered
inventory cannot establish VMID availability; use the dedicated cluster next-ID
check with the explicit candidate. Provider failure or incomplete metadata is
unknown, never proof of availability or compatibility. Explicitly retain unknowns
for template revision identity, DHCP, guest readiness and execution permissions.

The response identifies its plan hash, reserves nothing and enables no creation.
The native client hides results after input changes and must distinguish a newly
computed plan from an older preview. Later execution will require fresh provider
observations and approval of a concrete disposable target. Default verification
uses localhost TLS fixtures, without runtime credentials or real infrastructure.
