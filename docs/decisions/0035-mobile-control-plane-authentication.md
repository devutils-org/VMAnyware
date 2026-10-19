# ADR 0035 — Single-operator API token for the first mobile journey

Date: 2026-10-18
Status: Server middleware/configuration tested locally; remote client/TLS pending.

A phone cannot use the existing port-only localhost connection. Before remote
client access, protect the complete API with an explicitly configured independent
VMANYWARE_ACCESS_TOKEN. Keep the service loopback-only and local development
unchanged when the token is absent. Remote TLS termination and native endpoint/
token profiles are the next integration step, not implied by this middleware.

Accept one Authorization bearer header with a bounded URL-safe token. Reject
missing, wrong, malformed or duplicate headers uniformly, including unknown paths
and health. Invalid/empty configured tokens fail startup before database or socket
creation. Fixed error text, safe diagnostics and redacted Debug never expose the
token. This single operator token grants API access; target-specific confirmations,
capability checks and durable mutation boundaries still apply independently.

Retain a SHA-256 verifier and compare request hashes with subtle::ConstantTimeEq.
subtle 2.6.1 already exists in the lockfile; the direct dependency is a justified
security primitive with no newly resolved crate/version. Runtime token creation/
rotation/deployment is not performed by this change. A future remote endpoint must
use verified HTTPS and must never send tokens in URLs or persist them in frontend
storage. This does not implement users/roles or Internet-facing deployment.

Three targeted router/config tests and one real isolated binary test pass. They
cover routes/writes/fallbacks, invalid/ambiguous headers, valid access, unchanged
local mode, safe logs and fail-closed startup. Scoped library/binary/auth-test Clippy
passes. Test processes use cleared environments, a fixture token and owned temporary
SQLite/loopback only; no Proxmox request, real credential or infrastructure change.
