# 0018 — VMAnyware product and technical names

Date: 2026-10-18
Status: accepted

## Decision

Use VMAnyware as the sole repository/product name, vmanyware for package,
namespace and file identifiers, and VMANYWARE for environment variables.
Rename Rust/npm packages and lockfile entries together with their imports,
commands, fixtures, YAML API namespace and Tauri product/bundle identifier.

## Consequences

This is a development-stage breaking configuration change. Use the current
.env.example and examples: old environment names and YAML API versions are
not accepted as aliases. Existing persisted definitions containing the former
API namespace need explicit export/update/reimport before reuse. The default
SQLite filename changes to vmanyware.sqlite; no automatic database migration
or deletion is performed.

The Tauri identifier is now dev.vmanyware.client. Existing application data
and persisted SSH trust remain in the previous app directory; there is no
implicit trust transfer. Reconfirm independently verified host keys in the new
application, or design a separate explicit migration before distribution.

Historical infrastructure identifiers in repository documents are normalized
to current branding. External lab resources and ignored local records were
not mutated. Resolve actual identifiers by VMID/local records before operations,
particularly certificate hostnames and snippet bindings. Historical documents
must not be treated as exact transcripts of those identifier strings.
