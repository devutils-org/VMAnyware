# Normal macOS startup smoke

Status: procedure prepared; not executed in the current Linux session. This
checks the normal application entry point, separately from `terminal_smoke`.
It does not validate SSH authentication, a real guest or distribution.

## Prerequisites and isolation

Use a macOS graphical session with the repository's pinned Rust toolchain,
Node/npm and Apple CommandLineTools. Run `make doctor` first. Use the existing
locked dependencies; if they are absent, run `npm --prefix apps/client ci`.

Check for a project development listener with
`lsof -nP -iTCP:1420 -sTCP:LISTEN`. Reuse or stop only an identified owned
development session before proceeding. Leave existing API processes alone.

Launch from the repository root:

```sh
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID npm --prefix apps/client run tauri dev -- --config '{"identifier":"dev.pocketlab.startup-smoke"}'
```

The configuration override selects a distinct application identifier; it does
not edit the checked-in configuration. The normal macOS app-local data resolver
should therefore select `~/Library/Application Support/dev.pocketlab.startup-smoke`
rather than the regular application's directory. Confirm the resolved location
before claiming app-data isolation. Use a fresh identifier suffix if this test
identifier has already been used. This isolation procedure itself still needs
macOS verification, including WebView storage behavior.

The native startup source constructs the host/agent/session managers without
listing agent identities. Removing the agent environment variables also prevents
this process from discovering a personal agent through them. Do not inspect SSH
hosts, list identities or open a terminal during this smoke. Do not inspect
personal credential files or delete existing application data.

## Acceptance walkthrough

1. Record macOS/architecture, tool versions, revision and exact test identifier.
   Confirm that the normal PocketLab window opens without a startup error.
2. Before clicking anything, confirm that local service status is unchecked and
   no inventory operation, SSH inspection or identity listing has started.
3. Click **Check native bridge**. Record the actual Rust result `macos`.
4. Choose a loopback port with no listener. For example, check port 18080 with
   `lsof -nP -iTCP:18080 -sTCP:LISTEN`; if occupied, choose another unused port.
   Enter that port under **Local service port**. Do not stop its owner.
5. Click **Check local service**. Expect **Local service unavailable.**, a safe
   error and **Retry service check**; the window must remain usable. Record the
   observed error text, without substituting a simulated IPC result.
6. Click **Retry service check** with the same unused port. Expect the same
   bounded failure and an enabled retry action after completion.
7. Change the port. Expect the previous status/error to reset to unchecked.
   No request should start merely because the field changed. Repeat the check
   on the verified unused port to confirm that the action remains usable.
8. Close the window and stop the owned development command with Ctrl-C if it
   remains running. Confirm that its native process and port 1420 listener end.
   Retain test data until its ownership/location has been verified; no cleanup
   of normal application data is part of this procedure.

Do not click **Connect**: that action requests provider inventory. A healthy
API/recovery walkthrough is a separate task and is not required for this
unavailable-service slice.

## Evidence to record

Update `STATE.md` with actual launch/bridge/UI outcomes, the isolated data
location, observed retry/reset behavior and owned-process cleanup. Record any
failure and leave the corresponding criterion open. Visual observations prove
UI behavior; source review and existing transport tests support the absence of
automatic agent access, but this walkthrough alone does not measure socket
accesses. Do not claim instrumented zero-agent-access evidence from observation.
