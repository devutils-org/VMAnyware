# Interim Phase 0 evidence and remaining risks

This is an interim report, not a Phase 0 exit decision. The application is a
local macOS preparation tool; a complete real-host environment workflow has not
been demonstrated. See `STATE.md` for current counts and commands.

| Area | Evidence available | Remaining gate |
| --- | --- | --- |
| Native macOS foundation | Compiled development/release executable; user confirmed native bridge | Manual walkthrough of newly added screens; signing/installers |
| Definitions | Strict YAML, deterministic hashes, SQLite migration/reopen, native HTTP and validation tests | Editing/versioning and large-catalog pagination are future product work |
| Provider transport | Local Proxmox 9.2.2 Rust probe with explicit CA/hostname and dedicated privilege-separated token; bad token/CA/hostname rejected; effective privileges empty; fixture deadline/size tests | Scoped template/storage ACLs and provisioning permissions |
| Inventory/preflight | Live Rust version/online-node reads and empty visible VM list on disposable pve-lab; HTTP/TLS fixtures, explicit catalog and unknowns | Known source template, scoped visibility and live preflight |
| Provider task status | Validated task identity, exact success semantics, bounded polling fixtures | Durable linkage/reconciliation and real provider task |
| Local process ownership | Real process lock/restart and signal tests, five-second drain bound with stalled HTTP body | Service packaging and installed-service operational experience |
| Diagnostics | Real binary tests for allowed fields and sensitive-input redaction | Operational experience on an installed service |
| CI | Pinned workflow, local command checks and actionlint | First actual GitHub/Linux run; native/mobile runners |
| VM provisioning | Explicit plan and template preparation procedure | Clone/configuration implementation plus approved disposable-host tests |
| Snapshot/restore | Declared intent only | Storage capability evidence, implementation and separately approved restore |
| Desktop SSH/PTY | Real Tauri/IPC/bundled-renderer/disposable-agent/shell smoke; native output-pressure and isolated WebKit fixtures | Normal startup, real guest/OpenSSH/agent interoperability and physical keyboard behavior |
| Mobile | No runtime evidence | SDKs, devices/simulators, SSH crypto/PTY build and lifecycle checks |

## Decisions that remain open

Keep the root workspace small until a working use case justifies a dependency.
An experimental SSH fixture is not a commitment to a mobile-capable SSH stack.
The existing single provider worker should not become a generic executable job
framework ahead of real clone/task behavior.

Before enabling creation, persist the intended target and provider task identity;
model an ambiguous timeout or process interruption without blindly replaying a
mutation. Re-observe VMID/ownership/provider state before retrying. A preflight
hash is not authorization and does not reserve a VMID or freeze the template.

Before reporting an environment ready, independently verify Cloud-Init, guest
addressing, host-key trust and authenticated SSH. A provider `OK` or successful
VM start is insufficient. Packages and snapshot requests in YAML remain desired
configuration until execution and readiness are implemented and observed.

Before remote service exposure, implement and test control-plane authentication
and transport configuration. The current service stays on loopback; the remote
Proxmox connection uses verified HTTPS over LAN/VPN. Do not compensate for
missing certificates or ACLs by disabling verification or using an admin token.

The next architecture decision should cite actual disposable-host observations
and the isolated desktop SSH results, then explicitly bound mobile support.
Keep unresolved risks visible rather than treating source code, fixture tests,
compilation and real infrastructure evidence as interchangeable.
