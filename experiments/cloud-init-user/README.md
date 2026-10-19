# Offline Proxmox scalar-user compatibility experiment

This experiment addresses the exact warning observed in the disposable Debian
13/Proxmox 9.2.2 clone. It is outside the application dependency graph. It never
connects to Proxmox, executes user-data, changes a guest or declares readiness.

Proxmox emits `user: debian` alongside `users: [default]`. Cloud-Init 25.1.4's
normalizer converts that string to a name mapping but records a recoverable
deprecation. Supplying `user: {name: debian}` takes the same normalization path
without the scalar-string deprecation. This is a narrow compatibility candidate
for the tested version, not a general Cloud-Init generator or future API promise.

`convert.py` accepts only the observed passwordless/default-user shape and a
matching explicit expected username. It refuses unknown top-level features,
passwords, package upgrades and unexpected user settings. It preserves other
parsed values, including the SSH public keys and hostname. Output is a complete
candidate user-data document, created exclusively with mode 0600. Errors do not
print the input payload. Comments/formatting are not preserved.

`check.py` executes the actual upstream 25.1.4 normalizer and schema validator:

- Original scalar emits the expected deprecation; converted mapping emits none.
- Normalized users/groups are identical, including requested username, inherited
  sudo/groups/shell/locked password and default-user marking.
- Other generated configuration/public keys are unchanged and schema-valid.
- Unsupported features/username mismatches fail, and an existing output is never
  overwritten. The test distribution defaults are a fixture, not a running guest.

## Reproduce without application dependency changes

From the repository root, use a fresh ignored directory. Downloads/venv remain
local; the source archive hash was recorded from the official HTTPS download in
this session (no detached signature verification is claimed). Upstream Python
requirements are installed only into this experimental venv, never the system
or product. Exact versions used locally are recorded in ignored
`.lab/cloud-init-compat/tested-python-packages.txt`; dependencies resolved on a
later run may differ.

```sh
set -eu
mkdir -p .lab
CLOUD_USER_DIR=$(mktemp -d "$PWD/.lab/cloud-user.XXXXXX")
curl --fail --show-error --location --proto '=https' --proto-redir '=https' \
  --output "$CLOUD_USER_DIR/source.tar.gz" \
  https://codeload.github.com/canonical/cloud-init/tar.gz/refs/tags/25.1.4
python3 - "$CLOUD_USER_DIR" <<'VERIFY'
import hashlib, sys, tarfile
from pathlib import Path
root = Path(sys.argv[1])
archive = root / "source.tar.gz"
assert hashlib.sha256(archive.read_bytes()).hexdigest() == "fa70a77fc3cd3167a051e9ab04af4d4f56d3ffa0deb320735c889a6a367d3a3d"
with tarfile.open(archive) as source:
    source.extractall(root, filter="data")
VERIFY
python3 -m venv "$CLOUD_USER_DIR/venv"
"$CLOUD_USER_DIR/venv/bin/pip" install -r "$CLOUD_USER_DIR/cloud-init-25.1.4/requirements.txt"
PYTHONPATH="$CLOUD_USER_DIR/cloud-init-25.1.4" \
  "$CLOUD_USER_DIR/venv/bin/python" -B experiments/cloud-init-user/check.py
```

Requires Python with venv/pip and tarfile's data extraction filter. These commands
were executed on Linux/Python 3.14 with PyYAML 6.0.3 and upstream Cloud-Init
25.1.4 source. No result for another Python/Cloud-Init version is implied.

Convert a fresh local generated document only after reviewing its identity:

```sh
"$CLOUD_USER_DIR/venv/bin/python" experiments/cloud-init-user/convert.py \
  /path/to/generated-user-data.yaml /path/to/new-candidate.yaml --expected-user debian
```

This writes a file only; it does not upload it or change a VM. Never reuse a
candidate across clones: hostnames, public keys and generated metadata belong
to its source VM. A custom Proxmox `cicustom user=` document replaces generated
user-data completely; subsequent ciuser/sshkeys changes are not automatically
merged into that file. Regenerate and verify the binding before every use.

## Actual evidence and remaining gate

The checker passes with synthetic passwordless data. The same conversion was
also applied offline to the stopped clone 201's real generated user-data:
strict schema passes, normalized identity/default settings are equal, the exact
public key is retained and the scalar deprecation disappears in the normalizer.
The original/candidate files are ignored, not repository fixtures.

A separate, explicitly approved fresh-clone 202 test applied a target-specific
snippet and passed first-boot Cloud-Init exit 0, done, with no fatal/recoverable
errors, strict SSH and resource checks. See [actual infrastructure evidence](../../docs/cloud-init-compatibility.md).
The checker itself remains offline and does not declare guest readiness.
The proof is scoped to the pinned versions and observed input shape. Other
warnings must remain visible; do not patch packages or suppress deprecations.

Sources: [Proxmox generator](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuServer/Cloudinit.pm),
[Cloud-Init 25.1.4 normalization](https://github.com/canonical/cloud-init/blob/25.1.4/cloudinit/distros/ug_util.py),
[Cloud-Init user configuration](https://docs.cloud-init.io/en/latest/reference/modules.html#users-and-groups),
[return codes](https://docs.cloud-init.io/en/latest/explanation/return_codes.html).

## Image-default-user alternative

`default_user.py` prepares an offline candidate without the legacy `user` field,
requiring supplied image-default account evidence that matches the declared user,
is password-locked and contains no inherited authorized keys/password. Other seed
fields remain unchanged. Wrong or absent evidence is rejected. This is not a guest
probe or an application mode; supplying a dictionary does not attest an actual VM.

`check_default_user.py` runs only this alternative against the same pinned upstream
25.1.4 code. It renders the upstream Debian reference configuration (not an actual
Debian image), compares scalar, named and default-user normalization, validates the
candidate schema, and exercises the actual SSH module's user/key selection. Host
key handling, credential application and other side-effect entry points are mocked;
no SSH files are scanned, users created or credentials applied. A deliberately
modified image default resolves to a different account and is refused by the
candidate's evidence requirement.

```sh
PYTHONPATH="$PWD/.lab/cloud-init-compat/cloud-init-25.1.4" \
  .lab/cloud-init-compat/venv/bin/python -B \
  experiments/cloud-init-user/check_default_user.py
```

For a fresh environment, use the earlier source/hash/venv preparation and substitute
that directory. The new check does not require rerunning the unchanged converter
suite. It proves the narrow upstream normalization/schema/SSH-selection behavior,
not installed template defaults, first-boot module completion, DHCP or readiness.
An application implementation must explicitly bind image-default evidence to its
catalog/template before omitting ciuser; silently assuming every image uses debian
would change the requested account. Existing explicit-user behavior is unchanged.
