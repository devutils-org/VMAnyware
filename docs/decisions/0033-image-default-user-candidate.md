# ADR 0033 — Image-default-user compatibility candidate

Date: 2026-10-18
Status: Offline upstream proof passed; explicit product attestation/mode pending.

The [Proxmox generator](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuServer/Cloudinit.pm)
omits the legacy user field when ciuser is absent and retains users=[default].
The [Cloud-Init 25.1.4 normalizer](https://github.com/canonical/cloud-init/blob/25.1.4/cloudinit/distros/ug_util.py)
then selects the distribution's default account. This can avoid the scalar
compatibility issue without introducing remote snippet writes, but only when that
image default is the exact declared guest user. This is an inference from generator
source plus offline Cloud-Init execution, not real Proxmox generation/boot evidence.

Add an isolated candidate/check to the existing experiment. Require supplied,
matching non-root password-locked image-default evidence, with no inherited keys
or password. Reuse the strict existing seed-shape converter, then remove only user;
retain users/default, hostname, top-level authorized keys and other seed fields.
An absent/mismatched image default must fail rather than provisioning another user.

The new check renders the actual upstream Debian reference template and runs the
actual pinned normalizer/schema validator. Scalar, named-user and omitted-user
forms normalize identically with that matching default. Only scalar records the
known deprecation. The actual SSH module chooses the declared user and exact keys;
credential application, host-key handling, subprocess and file side effects are
mocked. Modified default-user evidence changes normalization and is rejected.

This establishes a useful compatibility strategy, not installed-image evidence.
Before changing product behavior, add explicit server-catalog opt-in/evidence bound
to the mapped template/version and declared guest user. Existing mappings must
preserve their inputs/hash and explicit-user behavior. Never infer the actual
image default solely from a distribution name or this reference configuration.
An operator attestation remains a declaration; strict real-template/first-boot
verification must remain separate and guest readiness must still fail on errors.

The focused new Python check passes on the existing isolated Cloud-Init 25.1.4
venv/source. No existing unrelated Rust/frontend/native or converter suite was
rerun. No product dependency, catalog behavior, provider write, installed package,
credential or real VM changed. Strict readiness and actual template proof remain
open. The earlier corrected-snippet lab recipe is still historical real evidence;
this alternative does not retroactively claim that lab used the default-user path.
