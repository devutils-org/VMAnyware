"""Opt-in semantic check against the actual upstream Cloud-Init 25.1.4 code."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

import yaml
from cloudinit import lifecycle, version
from cloudinit.config.schema import validate_cloudconfig_schema
from cloudinit.distros.ug_util import normalize_users_groups

from convert import UnsupportedUserData, convert

assert version.__VERSION__ == "25.1.4", "Check requires Cloud-Init 25.1.4 source"


class DistroFixture:
    def get_default_user(self):
        return {"name": "image-default", "groups": ["adm", "sudo"], "lock_passwd": True,
                "sudo": ["ALL=(ALL) NOPASSWD:ALL"], "shell": "/bin/bash"}


original = {"hostname": "fixture-compat", "manage_etc_hosts": True, "fqdn": "fixture-compat.invalid",
            "user": "debian", "users": ["default"], "chpasswd": {"expire": False},
            "ssh_authorized_keys": ["ssh-ed25519 fixture-public-key-for-offline-normalization"]}
source = "#cloud-config\n" + yaml.safe_dump(original)
converted = yaml.safe_load(convert(source, "debian"))
with patch.object(lifecycle, "deprecate", wraps=lifecycle.deprecate) as deprecated:
    before = normalize_users_groups(copy.deepcopy(original), DistroFixture())
    assert deprecated.call_count == 1
    assert deprecated.call_args.kwargs["deprecated"] == "'user' of type string"
with patch.object(lifecycle, "deprecate", wraps=lifecycle.deprecate) as deprecated:
    after = normalize_users_groups(copy.deepcopy(converted), DistroFixture())
    assert deprecated.call_count == 0
assert before == after
assert set(after[0]) == {"debian"} and after[0]["debian"]["default"]
assert after[0]["debian"]["lock_passwd"] and after[0]["debian"]["sudo"] == ["ALL=(ALL) NOPASSWD:ALL"]
assert {k: v for k, v in converted.items() if k != "user"} == {k: v for k, v in original.items() if k != "user"}
validate_cloudconfig_schema(converted, strict=True)
for data, expected in ((original, "other-user"), (dict(original, password="fixture-password"), "debian"),
                       (dict(original, runcmd=["fixture-command"]), "debian")):
    try:
        convert(yaml.safe_dump(data), expected)
    except UnsupportedUserData:
        pass
    else:
        raise AssertionError("Unexpected input accepted")
with tempfile.TemporaryDirectory(prefix="pl-cloud-user-") as directory:
    root = Path(directory);src = root / "source.yaml";dst = root / "candidate.yaml"
    src.write_text(source)
    command = [sys.executable, str(Path(__file__).with_name("convert.py")), str(src), str(dst), "--expected-user", "debian"]
    subprocess.run(command, check=True, capture_output=True)
    assert dst.stat().st_mode & 0o777 == 0o600
    first = dst.read_bytes()
    repeat = subprocess.run(command, capture_output=True)
    assert repeat.returncode == 1 and dst.read_bytes() == first
print(json.dumps({"cloudInitVersion": version.__VERSION__, "scalarDeprecations": 1,
                  "mappingDeprecations": 0, "normalizedUsersEqual": True,
                  "schemaValid": True, "unexpectedInputRejected": True,
                  "exclusiveOutput0600": True}, indent=2))
