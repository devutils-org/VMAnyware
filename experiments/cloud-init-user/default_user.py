"""Offline candidate requiring supplied image-default evidence, never a guest probe."""
import yaml

from convert import UnsupportedUserData, convert


def candidate(source: str, expected_user: str, image_default: dict) -> str:
    """Remove scalar user only when the supplied locked default account matches."""
    if (
        not isinstance(image_default, dict)
        or expected_user == "root"
        or image_default.get("name") != expected_user
        or image_default.get("lock_passwd") is not True
        or not isinstance(image_default.get("shell"), str)
        or not image_default["shell"].startswith("/")
        or "ssh_authorized_keys" in image_default
        or "passwd" in image_default
    ):
        raise UnsupportedUserData("Matching locked image-default evidence is required.")
    # Reuse the narrow passwordless shape checks; this does not run any user-data.
    data = yaml.safe_load(convert(source, expected_user))
    del data["user"]
    return "#cloud-config\n" + yaml.safe_dump(data, sort_keys=False)
