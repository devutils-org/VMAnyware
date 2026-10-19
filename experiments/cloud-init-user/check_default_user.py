"""Actual upstream normalization/schema/SSH selection, with all SSH effects mocked."""
import copy
import json
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import cloudinit
from cloudinit import lifecycle, templater, version
from cloudinit.config import cc_ssh
from cloudinit.config.schema import validate_cloudconfig_schema
from cloudinit.distros.ug_util import normalize_users_groups
import yaml

from convert import UnsupportedUserData, convert
from default_user import candidate

assert version.__VERSION__ == "25.1.4", "Requires the pinned Cloud-Init source"
source_root = Path(cloudinit.__file__).resolve().parent.parent
# Read public upstream source only; this is not /etc/cloud or an actual guest image.
reference = yaml.safe_load(templater.render_from_file(
    str(source_root / "config/cloud.cfg.tmpl"), {"variant": "debian", "prefix": None}
))
default_account = reference["system_info"]["default_user"]
assert default_account["name"] == "debian" and default_account["lock_passwd"]


class DistroFixture:
    def __init__(self, default):
        self.default = copy.deepcopy(default)

    def get_default_user(self):
        return copy.deepcopy(self.default)


original = {
    "hostname": "fixture-default-user", "manage_etc_hosts": True,
    "fqdn": "fixture-default-user.example.test", "user": "debian",
    "users": ["default"], "chpasswd": {"expire": False},
    "ssh_authorized_keys": [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    ],
}
source = "#cloud-config\n" + yaml.safe_dump(original)
converted = yaml.safe_load(candidate(source, "debian", default_account))
named = yaml.safe_load(convert(source, "debian"))
assert converted == {k: v for k, v in original.items() if k != "user"}
validate_cloudconfig_schema(converted, strict=True)


def normalized(data, default):
    with patch.object(lifecycle, "deprecate", wraps=lifecycle.deprecate) as deprecated:
        result = normalize_users_groups(copy.deepcopy(data), DistroFixture(default))
        return result, deprecated.call_count


legacy, legacy_warnings = normalized(original, default_account)
named_result, named_warnings = normalized(named, default_account)
default_result, default_warnings = normalized(converted, default_account)
assert legacy == named_result == default_result
assert (legacy_warnings, named_warnings, default_warnings) == (1, 0, 0)
assert set(default_result[0]) == {"debian"}
assert default_result[0]["debian"]["default"]
assert default_result[0]["debian"]["lock_passwd"]
assert default_result[0]["debian"]["shell"] == default_account["shell"]
assert default_result[0]["debian"]["sudo"] == default_account["sudo"]

# Execute the actual SSH module's user/key selection, never credential application.
# Disable host-key paths and replace every side-effect entry point on this path.
ssh_config = dict(copy.deepcopy(converted), ssh_deletekeys=False, ssh_genkeytypes=[],
                  ssh_publish_hostkeys={"enabled": False})
cloud = SimpleNamespace(distro=DistroFixture(default_account), get_public_ssh_keys=lambda: [])
with (
    patch.object(cc_ssh, "apply_credentials", autospec=True) as apply_credentials,
    patch.object(cc_ssh, "get_public_host_keys", side_effect=AssertionError("Unexpected host-key read")),
    patch.object(cc_ssh.util, "fips_enabled", return_value=False),
    patch.object(cc_ssh.util, "del_file", side_effect=AssertionError("Unexpected file deletion")),
    patch.object(cc_ssh.util, "write_file", side_effect=AssertionError("Unexpected file write")),
    patch.object(cc_ssh.subp, "subp", side_effect=AssertionError("Unexpected subprocess")),
    patch.object(cc_ssh.glob, "glob", side_effect=AssertionError("Unexpected SSH file scan")),
):
    cc_ssh.handle("ssh", ssh_config, cloud, [])
    apply_credentials.assert_called_once()
    selected_keys, selected_user, disable_root, _ = apply_credentials.call_args.args
    assert selected_keys == original["ssh_authorized_keys"]
    assert selected_user == "debian" and disable_root is True

# A modified image can resolve the same seed to another user: fail the evidence check.
wrong_default = dict(default_account, name="different-image-user")
wrong_result, _ = normalized(converted, wrong_default)
assert set(wrong_result[0]) == {"different-image-user"} and wrong_result != default_result
for evidence in [None, {}, wrong_default, dict(default_account, lock_passwd=False),
                 dict(default_account, ssh_authorized_keys=["inherited-key"])]:
    try:
        candidate(source, "debian", evidence)
    except UnsupportedUserData:
        pass
    else:
        raise AssertionError("Unproven image-default account accepted")
for changed in [dict(original, user="other-user"), dict(original, password="fixture"),
                dict(original, runcmd=["unexpected"]), dict(original, package_upgrade=True)]:
    try:
        candidate(yaml.safe_dump(changed), "debian", default_account)
    except UnsupportedUserData:
        pass
    else:
        raise AssertionError("Unexpected seed accepted")
print(json.dumps({
    "cloudInitVersion": version.__VERSION__, "reference": "upstream-debian-template-not-guest",
    "normalizedUsersEqual": True, "scalarDeprecations": legacy_warnings,
    "defaultUserDeprecations": default_warnings, "schemaValid": True,
    "sshSelectedDeclaredUserAndKeys": True, "imageDefaultMismatchRejected": True,
    "credentialApplicationMocked": True, "realGuestVerified": False,
}, indent=2))
