# Install the Linux control plane

This local package is a development artifact, not a release candidate. Build it
on the Linux architecture/distribution where it will run, or verify its dynamic
library requirements there. `BUILD.json` records architecture and the build host's
libc; it does not promise an older-glibc baseline or a supported distribution matrix.
Python 3.11+, Cargo/Rust and a C toolchain are build tools, not server requirements.
The server host needs systemd, a POSIX shell, `install` and the binary's shared
libraries. No Proxmox credentials or VM are needed to check an empty installation.

## Build and review

From the repository, run `make package-server`. The allowlisted archive and its
SHA-256 companion appear in `.lab/packages/`. It contains the release server,
unit/sysusers files, configuration example, installer, licenses and these
instructions. It never packages local databases, credentials or lab records.

Verify the outer checksum with `sha256sum -c <archive>.sha256`, extract the archive,
then run `sha256sum -c SHA256SUMS` inside its directory. Inspect `BUILD.json`,
`vmanyware.service`, `server.env.example` and `install.sh`. `ldd bin/vmanyware-server`
shows this build's dynamic requirements. Checksums detect corruption; they are not
a signature or independent publisher authentication.

To review installation paths without changing the host:

```sh
mkdir -p /tmp/vmanyware-staging
./install.sh --root /tmp/vmanyware-staging
```

The installer copies files only. Existing `server.env`, SQLite state and service
runtime are untouched. It does not create a service account, generate credentials,
reload systemd, configure TLS or start/enable anything. For an explicitly selected
server host, host installation is `sudo ./install.sh --root /`.

## Request capacity

The production server limits all routes before authentication with one process-wide
budget: burst 120, refill 20 requests/second, and 32 concurrent handlers until response
headers. HTTP 429 includes `Retry-After: 1`; wait before retrying. Keep the original
idempotency key/body after an uncertain mutation or read the known operation ID.
The budget resets on restart and can be exhausted by unauthenticated requests.
Configure HTTPS proxy connection/body/time limits for the actual deployment as well.
`make check-server-package SERVER_PACKAGE=/absolute/path/to/archive.tar.gz` verifies
authentication, persistence across restart, and admission/refill on the staged release
binary without installing a host service or contacting a provider.

## Configure the selected host

After reviewing the target and files:

```sh
sudo systemd-sysusers /usr/lib/sysusers.d/vmanyware.conf
sudo install -m 0600 /etc/vmanyware/server.env.example /etc/vmanyware/server.env
sudoedit /etc/vmanyware/server.env
```

Use a unique 32–256 character alphanumeric/underscore/hyphen access token. For
example, generate a fresh value with `openssl rand -hex 32` and enter it in the
root-owned configuration; do not put real tokens in repository files, command
arguments, screenshots or support logs. The service refuses startup with an empty
token and the binary rejects malformed tokens. Back up an existing configuration
before changing it; the copy command above is for a new installation only.

Keep all five write flags false initially. The service listens only on
`127.0.0.1:8080`; changing `VMANYWARE_PORT` does not change the loopback address.
State lives in `/var/lib/vmanyware`, created for the dedicated account by systemd.
The unit permits writes to this state directory while protecting system/home
paths. The configuration file is read by the service manager; it remains mode 0600.
The configuration directory is mode 0755 so the service account can traverse it
to read public CA/catalog files, which should be mode 0644. Never place a secret
in a world-readable file.

Start only after reviewing the configuration:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now vmanyware.service
sudo systemctl status vmanyware.service
sudo journalctl -u vmanyware.service --since today
```

The unit uses `Type=exec`, so starting it proves process execution, not application
readiness. Confirm the authenticated `/api/v1/health` response before use. Missing
or wrong authorization must return 401. Avoid putting the bearer token directly
in a command argument; use your client or an appropriately protected HTTP client
configuration. Never share actual HTTP client configuration or environment dumps.

The unit's process/directory/environment behavior follows
[systemd.service](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html)
and [systemd.exec](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html).
Actual service-manager startup and its sandbox must be tested on each supported
host; a staged process smoke does not prove the unit ran under systemd.

## HTTPS and the mobile client

Keep the backend loopback-only. Put a verified TLS reverse proxy on this host in
front of it, preserving Authorization and `X-VMAnyware-Authenticated`. For example,
a separately installed Caddy instance can use:

```caddyfile
vm.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

Replace the hostname with the selected server's DNS name and configure its routing,
certificate issuance and firewall explicitly. Follow the official
[Caddy reverse-proxy quick start](https://caddyserver.com/docs/quick-starts/reverse-proxy).
This package does not install/configure Caddy or open firewall ports. Do not expose
the backend HTTP port or Proxmox directly. For a private CA, distribute its public
certificate over a trusted channel and enter that CA in the native client. Never
paste a TLS private key or bypass hostname/certificate verification.

In the native app select remote mode, enter the HTTPS address and control-plane
access token, optionally the public CA, connect and load the workspace. Without a
configured provider/catalog an empty server is not ready to allocate a VM. Configure
the verified Proxmox URL, public CA, dedicated token and reviewed template catalog
using `server.env.example`, the packaged `templates.yaml.example` and
`docs/provisioning-plan.md` in the source repository. The catalog example is not
verified against your host and intentionally does not attest an image-default
user; review and configure that mapping before enabling creation.
Only enable provisioning/lifecycle after validating the concrete disposable target
and least-privilege execution. Snapshot, restore and deletion have separate flags.
The package does not make a source-prepared mobile app into an accepted mobile
release; actual client artifact/platform acceptance remains separate.

## Restart, upgrade and restore

Stop the service before copying or replacing its binary. Before an upgrade, stop
it and preserve the entire state directory plus root-only configuration, catalog
and public CA. A stopped copy includes SQLite sidecars and the lock pathname;
copying only the main database while running can lose state. Protect backup tokens
as secrets. The application lock prevents a second process from using the same DB.

Install the new reviewed package without overwriting `server.env`, start the service
and inspect health, definitions and saved operations. Migrations run at startup.
Do not run an older binary against a newer schema: rollback requires the matching
old binary AND the stopped pre-upgrade state backup. To restore, stop the service,
restore the complete selected backup with service-account ownership and mode 0700
on its directory, then start and inspect saved state. Never automatically retry an
uncertain provider mutation after recovery; inspect the existing operation.

The source `check-server-package.py` now exercises this stopped-copy procedure with
the exact staged package: create a definition and a read-only inventory operation
that fails safely without a provider, stop and copy the complete temporary state,
restart and add a later definition, stop and restore the earlier copy, then start
again. It verifies identical saved definition/operation/history and idempotency-key
lookup, absence of the later definition and preservation of copied files/private
directory mode. This covers same-version database restoration under the current
OS user, not systemd ownership/sandbox behavior, configuration-secret backup,
older-version migrations, live copying or provider/guest rollback. The existing
archive's instructions describe the same stopped-copy procedure.

For an isolated previous-schema check, run from the source checkout:

```sh
python3 scripts/check-server-package.py /absolute/path/to/archive.tar.gz --previous-schema
```

This constructs schema 14 from the checked-in migration SQL, with a saved definition
and terminal inventory operation, and lets the exact packaged process migrate it
to schema 15. It checks retained rows/API/hash/revision/history/idempotency, SQLite
integrity and foreign keys, new lineage triggers, restart and stopped-copy restore.
The pre-upgrade database copy remains unchanged. Other instance/mutation tables
in this fixture are empty. No older application binary runs, so this is not a
published-version upgrade, app upgrade, populated mutation-journal migration or
old-binary rollback test. Use the normal package check for clean-database behavior.

To check downgrade protection independently:

```sh
python3 scripts/check-server-package.py /absolute/path/to/archive.tar.gz --future-schema-only
```

After archive/staging checks, this creates a disposable future-schema-16 database
with witness rows, then attempts two starts of the current schema-15 package.
Both must exit with the fixed storage failure before listening, preserving exact
database bytes, version, schema and rows. The application may create its companion
lock pathname, but releases the lock on failure. This does not test an actual future
release, concurrent database writers or recovery from corruption. Do not decrement
a real database's version to force an older binary to open it; restore the matching
stopped pre-upgrade backup and binary instead.

## Troubleshooting and uninstall

A failed pre-start check usually means no token was configured. Invalid token,
provider or catalog values fail startup with fixed redacted events. An unexpected
worker exit stops the service; inspect the journal and existing operation history
before restarting or enabling writes. TLS failures require correct CA/hostname and
routing, not verification bypasses. A database-lock error means another process
may still own the DB; stop/identify that process rather than deleting state.

To remove executable/service files after an explicit decision on the selected host:
stop/disable `vmanyware.service`, remove its unit and
`/usr/local/lib/vmanyware/vmanyware-server`, then reload systemd. Configuration,
`/var/lib/vmanyware`, backups and the service account are retained for recovery.
Delete those separately only after reviewing the concrete data to remove. This
package has no destructive uninstall command.
