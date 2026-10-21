# 0069 — Explicit Linux server artifact and staged installation

Status: accepted for local Linux packaging; target service-manager acceptance open.

Deliver a small allowlisted archive containing the actual release server binary,
systemd/sysusers files, configuration/template examples, copy-only installer,
licenses, build metadata and installation/recovery instructions. The archive is
architecture-labelled, with inner/outer SHA-256 checksums. It is not signed, not a
release candidate and not a generic older-glibc build. No private workspace file,
database, credential or lab record is included. Building requires no extra crates.

Installation requires an explicit absolute root. Staging and reinstalling copy
files without creating an account, changing server.env/state, generating a token,
configuring a proxy or starting/enabling a host service. The administrator reviews
and performs those separate operations on the selected host. There is no automatic
or destructive uninstall. Upgrade/rollback instructions retain full stopped state
and require a matching old binary/state backup rather than reverse migrations.

The service runs as a dedicated static account with persistent restricted state,
protects system/home paths and requires a nonempty access token before execution.
The binary independently validates the token. EnvironmentFile is root-only; all
five mutation flags default false and the backend still binds loopback only. TLS
termination remains an explicit separately operated proxy. The packaged template
example does not invent a verified template-user attestation or provider ACLs.

The targeted artifact check validates both checksum layers, isolated staging and
reinstall preservation, unit syntax with staged ExecStart, the unit's shell auth
preflight, actual packaged process authentication/health/clean SIGTERM, retained
definition after restart and malformed-token failure before database creation.
No host unit/account, credential or provider is modified. Unit syntax and process
checks do not prove sandbox behavior or service-manager launch on a target host.

Current evidence is a locally built Linux x86_64/glibc artifact, not mobile client
packaging, real Proxmox execution or a release/platform claim. The next delivery
work is a testable Android client artifact, followed by selected-host service/TLS
acceptance and the concretely approved disposable VM allocation/SSH journey.
