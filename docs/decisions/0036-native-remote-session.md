# ADR 0036 — In-memory remote HTTPS connection

Date: 2026-10-18
Status: Native transport/form tested locally; remote VM workflow and phone runtime pending.

Add a separate remote connection mode with an HTTPS origin and independent API
token. Retain the profile only in native session memory; never save the token in
frontend storage. Reject URL credentials, path/query/fragment, cleartext, invalid
tokens, untrusted TLS, redirects and malformed health. Require the accepted bearer
authentication marker from the server middleware, not simply a 200 response.

Invalidate old profile generations on reconnect/disconnect and reject stale checks.
Return only connection ID and endpoint. The form clears the token on submit and
unmounts local VM controls in remote mode, so connection checks cannot redirect an
existing local action to another server. Remote VM actions are the next integration.
No generic HTTP passthrough or certificate-verification bypass is introduced.

Enable reqwest rustls for this actual HTTPS use case; existing resolved TLS crates
are reused. Three filtered native TLS/state tests, frontend production build,
focused Firefox simulated-IPC form checks and native/server library Clippy pass.
The server auth-marker response contract is covered by its targeted route test.
These complementary tests do not prove new-command IPC in a launched native app,
phone runtime, deployment or a created VM. Mobile key storage remains separate.
