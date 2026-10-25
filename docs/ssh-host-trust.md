# Persistent SSH host-key trust policy

This policy is shared by the native macOS host-inspection prototype and the
isolated SSH experiment. It covers explicit first-use confirmation, persistence,
endpoint scope, changed-key rejection and failure-safe storage. No personal SSH
configuration, known-host file or credential file is read or modified. Automated
tests write generated fixture public keys only to newly created test directories.
The [native inspection flow](ssh-host-inspection.md) describes the application UI
and its separate Rust-owned confirmation challenges.

## Trust flow

A server public key is evidence of what answered a connection, not proof that
the server is the intended host. A first observation must not authorize client
key authentication. The fixture closes the untrusted handshake and produces an
opaque confirmation containing the observed endpoint/key and fingerprint.
Simply observing or cancelling must not add trust.

Confirmation must match that exact observed fingerprint. A later connection
uses the persisted key, rechecks the actual server key during key exchange and
only proceeds to authentication on an exact match. A fresh key exchange is
required after approval; the server may have changed between probe and reconnect.
Automated tests model the approval call. The native UI adds explicit verification
and confirmation controls; its manual walkthrough is a separate verification step.

A changed key is a separate failure, not another first-use prompt. The first-use
confirmation API cannot overwrite an existing different key, including when
another process accepted a key after a challenge was created. There is no key
automatic rotation or trust reset. A changed key can only be replaced through
the separate explicit replacement operation (ADR 0093): the inspection returns
the previously trusted and newly observed fingerprints with a distinct native
challenge, and replacement requires echoing both fingerprints. Storage replaces
the pin only while the reviewed previous key is still pinned.

The native UI displays the intended host, port, key algorithm and
SHA-256 fingerprint and asks the user to compare it through an independent
trusted channel before an explicit decision. Its IPC keeps the exact observed
key and confirmation state in Rust rather than accepting an arbitrary public
key supplied by the WebView. The store API itself does not implement IPC
challenge lifetimes; the native manager adds that boundary. Private-key storage
and general connection settings remain outside this policy.

## Endpoint and storage boundaries

Trust is scoped to the requested host and port. Canonicalization must not merge
DNS aliases with resolved IP addresses or merge different ports. Host spelling
rules and accepted addresses are validated before storage or network access.
A matching public key on another endpoint does not silently authorize that host.
One key is pinned per endpoint; additional algorithms need a deliberate future
policy rather than accepting arbitrary alternatives.

The store contains public host keys and endpoint identifiers only. Although it
contains no authentication secret, host inventory is local information and the
managed file/directory are private. It is separate from OpenSSH `known_hosts`
and is not an import/export or compatibility implementation for that format.

Read-modify-write operations use a stable sidecar lock with a bounded wait and
atomically replace the data file from the same directory. Confirmation rereads the current state
under the lock before writing. Identical concurrent confirmations may succeed
idempotently; conflicting ones cannot overwrite the winner, and independent
hosts must not lose each other's updates.

Malformed, oversized, duplicate, unsupported-version or unreadable storage must
fail closed. It must not be silently treated as a new empty store. Only explicit creation
of a new managed directory initializes an empty store.
A missing/deleted data file in an existing directory is also a failure.
This protects against accidental corruption and cooperating concurrent writers;
it does not make the file trustworthy against an attacker controlling the user
account or parent filesystem. Read the experiment README for exact format,
limits, permissions and verified operating-system behavior.

## Verification boundary

`make check-ssh` is opt-in and uses generated loopback fixtures. The experiment remains outside the application workspace; native macOS checks
also exercise the shared trust source and host-inspection manager. The tests must show
that rejected unknown/changed keys never reach the authentication callback, and
that a stored approved key still works after reopening the store.

This is not yet a native terminal, persistent private-key storage, a production
SSH library choice, Windows/mobile evidence or real-guest interoperability proof.
Do not mark those roadmap milestones complete from store tests alone. See
[desktop experiment](../experiments/ssh/README.md),
[ADR 0008](decisions/0008-isolate-desktop-ssh-feasibility.md) and
[interim feasibility](feasibility.md).
