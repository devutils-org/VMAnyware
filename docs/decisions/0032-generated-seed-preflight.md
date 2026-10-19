# ADR 0032 — Read-only generated seed semantic preflight

Date: 2026-10-18
Status: Fixed generated-preview inspection tested locally; compatible generation/start integration pending.

The [upstream QEMU dump endpoint](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
reads automatically generated user/network data. It does not read custom snippets
or independently inspect the attached seed ISO. Require the exact applied
passwordless Cloud-Init settings first; that observer already rejects custom
overrides. Use fixed GET cloudinit/dump?type=user and type=network only, enclosed
by full stopped-target/settings observations with matching configuration digests.
The whole operation has a 20-second deadline; each YAML payload is limited to
64 KiB within the provider's existing bounded response handling.

Typed, unknown/duplicate-field-rejecting YAML parsing permits only the supported
passwordless shape: exact hostname/fqdn/user, default-user list, non-expiring
password policy, no password/commands, no implicit upgrades and exact canonical
public keys. Accept the ADR 0017 named-user mapping as a structurally compatible
candidate; recognize the exact scalar user as scalar_unsupported and return
compatible=false. Wrong users/keys, unknown fields and malformed data fail closed
with fixed errors, without returning parse diagnostics or raw payloads.

Support only the provider's generated network-v1 shape: one eth0 physical
interface with the exact retained MAC and one DHCPv4 subnet. Optionally accept one
bounded nameserver section with parsed IP addresses and bounded search domains.
Other interfaces, addresses, subnets, commands or versions fail closed. This is a
narrow template contract, not a general Cloud-Init schema validator.

The safe report contains configuration revision, SHA-256 payload hashes, named
versus unsupported scalar classification, structural compatibility and
explicit guestReady=false. It contains no raw keys, user-data or DNS addresses.
Compatibility means only this generated preview matches the supported semantics;
it proves neither full Cloud-Init schema/runtime behavior nor seed regeneration,
attached contents, default-user identity inside the image or guest readiness.
The provisioning start stage is not implemented/wired yet; future start handling
must require appropriate fresh compatibility and generation evidence.

The [current upstream generator](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuServer/Cloudinit.pm)
still emits a scalar when ciuser is present. The known pinned combination therefore
remains incompatible; a hypothetical named-user fixture is not a claim that this
provider now generates it. Investigate the image default-user path before adding
remote snippet transport, while preserving the declared user/key contract and
strict first-boot exit-zero/no-recoverable-error requirement.

Two focused local HTTPS tests pass, with multiple adverse cases: fixed GET paths,
stable revision, named versus scalar classification, safe reporting and rejection
of wrong users/keys/hostname/password policy, commands/password/upgrade/duplicate
fields, unexpected network/MAC/DHCP/DNS/version and a changed revision. Fixtures
use the upstream False spelling. Scoped library Clippy passes. No provider write,
schema/dependency change, real infrastructure operation or gate closure is implied.
