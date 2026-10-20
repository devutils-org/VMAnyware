# 0051 — Explicit handoff from ready creation to SSH

Status: implemented; browser interaction verified, integrated native journey open.

## Context

Remote creation and SSH worked as separate paths, but remote mode hid the local
SSH screen. A successful create result had no action to prepare its guest endpoint.
Guest initialization is not evidence of SSH host identity or authentication.

## Decision

Offer “Prepare SSH connection” only for succeeded operations with guestReady.
A fresh creation also supplies the username from its native retained plan. A
recovered operation supplies its recorded IPv4 address and asks the operator for
the account after host inspection; the recovery contract does not include a
trusted guest username. Default SSH port is 22 and the normal inspection form
allows an explicit edit.

The user must select the action. It mounts the existing host-inspection flow in
remote mode, prefills the endpoint/account and focuses its heading. It does not
inspect, trust, unlock, authenticate or open a terminal automatically. An unknown
host still needs independent fingerprint verification and explicit confirmation,
followed by a fresh handshake. A changed host retains the existing refusal.

Every target selection remounts that flow, including reselecting the same target,
so earlier challenges, identity selections and terminal state are discarded through
the existing cleanup. Connection replacement, disconnect and leaving remote mode
unmount the selected SSH target. No browser persistence, transport API, dependency
or trust-policy change is introduced.

## Evidence boundary

The targeted real React/Firefox fixture uses simulated IPC to check queued/failed
operations have no handoff, readiness selects nothing automatically, explicit
handoff fills the exact endpoint/account, unknown-host confirmation gates account
access, recovery does not guess a username, and reselection/disconnect clear state.
The existing application-key SSH browser scenario also passes after the optional
initial-account changes. Production frontend builds. These checks do not prove a
single native creation/readiness/SSH journey or real VM behavior; those gates
remain required and are the next integration task.

A later current-APK Android run now verifies the explicit exact-address/account
handoff after actual API/worker creation against a generated provider (ADR 0052).
The SSH endpoint is then explicitly overridden for the owned ADB tunnel. This
advances native integration without proving the synthetic guest is reachable.
