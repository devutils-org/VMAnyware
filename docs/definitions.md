# Environment definitions, version 1

Definitions are reusable desired configuration. Saving one never creates or
changes a VM. Provider/runtime state will be stored separately when provisioning
is implemented. Export and recreation do not restore previous disk contents.

Use [examples/kernel-lab.yaml](../examples/kernel-lab.yaml) as the full example.
Replace its demonstration public key. File paths to keys are deliberately not
accepted on the server: the client must send the public key contents, never a
private key. No files, YAML directives or shell commands are executed.

## Exact schema

Every object rejects unknown fields. Duplicate YAML fields and multiple YAML
documents are rejected. Definitions are limited to 64 KiB of UTF-8 YAML.

| Field | Rule/default |
| --- | --- |
| `apiVersion` | Required literal `pocketlab.dev/v1` |
| `kind` | Required literal `Environment` |
| `metadata.name` | Required, 1–63 lowercase letters/digits/hyphens; no leading/trailing hyphen |
| `metadata.description` | Optional, default empty string, maximum 1024 bytes |
| `spec.image` | Required image identifier, 1–64 lowercase letters/digits/dots/hyphens; explicit template mapping still required |
| `spec.resources.cpu` | Required integer, 1–64 |
| `spec.resources.memoryMiB` | Required integer, 256–262144 |
| `spec.resources.diskGiB` | Required integer, 4–4096 |
| `spec.provisioning.cloudInit` | Required `true` |
| `spec.provisioning.sshAuthorizedKeys` | Required 1–16 unique inline OpenSSH public keys, each a single line up to 8192 bytes |
| `spec.provisioning.packages` | Optional, default empty list, up to 128 unique package declarations of at most 128 bytes each |
| `spec.lifecycle.snapshotAfterProvisioning` | Optional, default `false`; the entire `lifecycle` object may be omitted |

Package declarations allow lowercase letters, digits and `+.-=:~`, beginning
with an alphanumeric character. They are declarations only; syntax does not
guarantee a package/version exists in the template's configured repositories.
Shell metacharacters/options are rejected. Provisioning/package installation
and automatic snapshots are not implemented yet.

Canonical export preserves struct field order and list order, expands defaults,
and removes comments/formatting. The SHA-256 digest is over this canonical UTF-8
YAML, so YAML formatting changes do not change the hash. Changing declared list
order does change the hash. This is configuration reproducibility, not a claim
that unpinned package repositories produce identical software.

## Local API

| Method | Route | Behavior |
| --- | --- | --- |
| `POST` | `/api/v1/definitions/validate` | Validate without saving; return name/hash/canonical YAML |
| `POST` | `/api/v1/definitions` | Validate and save a new definition; HTTP 201 |
| `GET` | `/api/v1/definitions` | List records sorted by name |
| `GET` | `/api/v1/definitions/{name}` | Read a saved record; missing names return HTTP 404 |

POST requires `Content-Type: application/yaml`. Requests carrying an `Origin`
header are refused: browser previews cannot mutate this local API. No CORS
permission is granted. The native application uses its Rust HTTP client.

Unknown fields/invalid values return HTTP 422 with safe `error.code`,
`error.message` and `error.field`; submitted values/parser diagnostics are not
echoed. Duplicate names return HTTP 409 and never overwrite the saved record.
Oversized bodies return HTTP 413. Validation failures never write to SQLite or
contact Proxmox.

Records include `name`, `yaml`, `hash`, `revision`, `createdAt`, `updatedAt`.
Revision is currently 1 because update/overwrite/delete are not exposed yet.
The initial native response limit is 2 MiB; pagination for large catalogs remains
future work. This initial API is for a small local homelab catalog.

## Persistence and recovery

`POCKETLAB_DATABASE` selects the SQLite file; default `pocketlab.sqlite` relative
to the server's working directory. The application does not load dotenv files.
The first launch applies `migrations/0001_definitions.sql` in a transaction and
sets `user_version=1`, then applies `0002_operations.sql` to reach version 2.
Reopening reuses the schema and existing records. A newer
unknown schema is refused rather than downgraded. Database errors are redacted.
SQLite work runs on blocking workers, not the asynchronous HTTP executor.

Back up the database before future migration upgrades. No private keys or
Proxmox credentials belong in this database. Database upgrades, backup tooling,
concurrent multi-process deployment and deletion/instance lifecycle await later
slices.

## Try without Proxmox

Start `make server`, then use **Save definition** / **Load saved definitions** in
the native application. Alternatively, from another terminal:

```sh
curl --fail-with-body -H 'Content-Type: application/yaml' \
  --data-binary @examples/kernel-lab.yaml \
  http://127.0.0.1:8080/api/v1/definitions/validate
```

Default tests use in-memory or temporary databases and never alter a user's
normal catalog or call a real Proxmox endpoint.
