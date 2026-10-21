# 0073 — Remove unused Android file sharing

Status: accepted for packaging; Android runtime acceptance remains open.

## Context and decision

The secret-storage review found that backup exclusions already cover cloud and
device transfer (ADR 0048), but the generated Android application still declared
an unused AndroidX FileProvider. Its path resource mapped all external storage and
all application cache with `path="."`. No current product code requests a URI from
this provider or implements file sharing/import/export. The provider was not
exported; this finding is unnecessary potential sharing authority, not proof that
another application could read the SSH identity or that a data leak occurred.

Remove the provider declaration and unused path resource. Keep Internet access,
launcher, TLS handling, private application identity/trust storage and existing
backup exclusions. Do not introduce an export directory or another abstraction
without a working product use case. A future explicit sharing workflow must review
its exact files/URI permissions and use narrow paths, never application secrets.

The APK packager now inspects the final compiled manifest, rejecting the unused
FileProvider, missing/incorrect backup flags, missing data-extraction rules and a
remaining broad file-path resource. Checking only source XML would miss manifest
merges or stale APKs. Both signatures/ABI/current native-library checks still apply.

## Evidence and limits

The strengthened packager rejects the previous real APK before copying its first
artifact because that binary retains the provider. Fresh Android builds and the
same packager supply the positive artifact evidence recorded in STATE.md.
This does not demonstrate runtime behavior or cloud/OEM transfer execution.
An app-controlled FileProvider removal does not make all library providers invalid;
AndroidX initialization providers can remain in the merged manifest.

Passphrase-encrypted SSH keys are still not an OS Keychain/Keystore integration.
This slice removes unused sharing authority; platform key-storage and lost-device
acceptance remain separate roadmap work. Do not regenerate the Android project
over the reviewed manifest/activity/resources.

## Reference

[Android: improperly exposed FileProvider directories](https://developer.android.com/privacy-and-security/risks/file-providers)
recommends avoiding broad ranges and unreviewed external-storage paths.
