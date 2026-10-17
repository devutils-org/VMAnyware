"""Offline conversion of the observed, passwordless Proxmox user-data shape."""
import argparse
import copy
import os
from pathlib import Path
import re

import yaml


class UnsupportedUserData(ValueError):
    """Fixed error: user-data payloads must not appear in diagnostics."""


def convert(source: str, expected_user: str) -> str:
    if len(source.encode()) > 65536 or not re.fullmatch(r"[a-z_][a-z0-9_-]{0,63}", expected_user):
        raise UnsupportedUserData("Unsupported user-data or expected user.")
    try:
        data = yaml.safe_load(source)
    except yaml.YAMLError:
        raise UnsupportedUserData("Cannot parse the supplied user-data.") from None
    allowed = {"hostname", "manage_etc_hosts", "fqdn", "user", "ssh_authorized_keys", "chpasswd", "users", "package_upgrade"}
    if (
        not isinstance(data, dict)
        or set(data) - allowed
        or data.get("user") != expected_user
        or data.get("users") != ["default"]
        or data.get("chpasswd") != {"expire": False}
        or data.get("package_upgrade", False) is not False
        or not isinstance(data.get("ssh_authorized_keys"), list)
        or not data["ssh_authorized_keys"]
        or not all(isinstance(key, str) and key.strip() for key in data["ssh_authorized_keys"])
    ):
        raise UnsupportedUserData("Unsupported passwordless Proxmox user-data shape.")
    result = copy.deepcopy(data)
    result["user"] = {"name": expected_user}
    return "#cloud-config\n" + yaml.safe_dump(result, sort_keys=False)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--expected-user", required=True)
    args = parser.parse_args()
    try:
        with args.source.open(encoding="utf-8") as stream:
            source = stream.read(65537)
        output = convert(source, args.expected_user)
        fd = os.open(args.destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            stream.write(output)
    except (OSError, UnicodeError, UnsupportedUserData):
        parser.exit(1, "Conversion refused; check input shape, expected user and unused output path.\n")
    print("Created offline compatibility candidate; no provider or guest was changed.")


if __name__ == "__main__":
    main()
