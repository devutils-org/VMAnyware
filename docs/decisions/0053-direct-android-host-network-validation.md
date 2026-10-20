# 0053 — Validate direct Android host-alias transport

Status: actual native direct HTTPS/SSH journey passes on the owned emulator.

## Context

Earlier attempts returned network-unreachable/connection failure and used an
explicit ADB reverse tunnel. Those failures did not prove a permanent emulator
routing limitation. After a fresh boot settled, routing showed a validated default
network and an owned TCP marker probe reached a loopback listener through 10.0.2.2.
No device routing, system trust, host firewall or app transport setting was changed.
The exact cause of the earlier transient failures is not established.

## Decision

Support explicit VMANYWARE_ANDROID_HTTPS_DIRECT=1 in the native harness. It uses
the documented emulator host alias and a generated certificate whose SAN includes
10.0.2.2. Keep the deliberately wrong-host certificate distinct so hostname refusal
is still tested. Direct HTTPS creates no ADB reverse mapping. The existing SSH
fixture already uses the host alias when its explicit tunnel opt-in is absent.

Gate direct-mode application testing with an independent bounded TCP marker probe
from the owned emulator to a fresh loopback listener. Boot completion alone is not
connectivity evidence. Record probe attempts/time and require an empty ADB reverse
list before the test and after the combined direct journey. A probe failure fails
the direct check; do not silently switch transport or bypass TLS/SSH verification.
ADB/CDP still drives the UI and collects diagnostics; it does not carry API/SSH
data in this mode. Existing tunnel mode remains explicitly labeled in reports.

## Evidence boundary

The current x86_64 APK passes confirmed creation through native verified HTTPS,
the actual production service/worker and a generated provider, followed by native
SSH trust/authentication/PTY through the host alias. The same Android key is checked
in Cloud-Init and used by the SSH server. Mutation counts remain stable across
terminal/application restart. A cold-boot repeat exercises the TCP readiness gate.

The provider and guest are simulated. The synthetic guest endpoint is explicitly
replaced by the host alias and ephemeral SSH port; no real guest is contacted.
This proves ordinary emulator TCP with verified HTTPS/SSH, not LAN/VPN deployment,
physical-device networking, public-root positive verification or real VM operation.
Earlier tunnel-only evidence remains historical; this result supersedes its claim
that direct host-alias reachability was still unproven on this owned emulator.

## Reference

[Android emulator address space](https://developer.android.com/studio/run/emulator-networking-address)
documents 10.0.2.2 as the development host loopback alias. No emulator networking
feature flag or legacy-network fallback was needed for this result.
