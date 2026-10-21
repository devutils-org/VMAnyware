# 0082 — Native acceptance of declared provisioning plans

Status: native protocol acceptance passes; explicit form/review delivery remains next.

## Decision

The native create/recreate Plan reader now accepts bounded Debian package
requests and the optional final snapshot. It applies the server transport grammar
without importing the server crate or introducing a domain abstraction. The two
implementations consume fixtures/package-declarations.json as a shared acceptance
contract, including pinned versions, architecture qualifiers, genuine g++ names,
invalid/removal/shell/release selectors, duplicates and non-ASCII input. Limits
remain 128 unique declarations of 128 bytes. Empty packages remain supported.

Snapshot intent is retained exactly as a boolean. Server creationEnabled is still
a separate gate; a syntactically valid plan does not grant dispatch permission.
The existing native retained-plan confirmation sends only its approval, whose
hashes bind declarations. Renderer edits cannot replace that private plan. An
explicit unknown-response retry uses the same request key and body. Recreation's
configuration comparison includes exact package values/order/version and snapshot
intent; only the current runtime capability is excluded from that comparison.
Saved recreation readers use the same validation without restoring consent.

No readiness contract is weakened: queued/accepted responses are not success,
scoped plan/guest proof remains necessary, and the server must finish all declared
stages before producing its succeeded result. This slice does not add controls to
the React form, launch the native application or rebuild delivery artifacts.
The existing recreation review includes the retained YAML; explicit feature
summaries and creation inputs are the next UI slice.

## Evidence

Thirteen focused native create/prepare/recovery tests pass, including three new
contract/bounds, exact plan comparison and actual generated-HTTPS preparation/
unknown-response retry tests. Twenty affected native recreation/recovery tests
pass. The server independently passes the same declaration corpus. All checks
are offline/owned fixtures; actual guest installation and current APK acceptance
remain open. Logs and next task are recorded in STATE.md.
