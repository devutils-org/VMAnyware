# Terminal client

The standalone `vmanyware` binary browses a VMAnyware control plane without
Tauri, Node, a WebView or Proxmox credentials. This first version is read-only:
machines, saved definitions, live power observations, recent operations and
recorded provisioning stages. The native application remains available for
creation, VM actions and SSH.

## Run

From the repository root:

```sh
cargo run --locked -p vmanyware-tui -- tui --endpoint https://control-plane.example
```

For a private CA, add `--ca /absolute/path/to/public-ca.pem`. The CA must be a
single public PEM certificate of at most 16 KiB. An explicit CA restricts trust
to that certificate; certificate dates and hostnames are still verified.
HTTP, URL credentials, paths, queries, fragments and redirects are rejected.

The connection screen lets you edit the endpoint and public CA path and paste
the control-plane access token into a hidden field. Press Enter to connect.
The token must meet the server's existing 32–256 character format. It is never
a Proxmox token. Endpoint, CA path and token are not saved by the TUI.

`make tui` opens the same screen. Optional environment inputs are
`VMANYWARE_ENDPOINT`, `VMANYWARE_CA` (public file path), and
`VMANYWARE_ACCESS_TOKEN`. Prefer the hidden field over a token in shell history;
there is deliberately no token command-line argument. Even with environment
inputs, Enter is required before the first request.

To install the standalone binary locally:

```sh
cargo install --locked --path apps/tui
vmanyware tui --endpoint https://control-plane.example
```

## Navigation

Mouse input is enabled by default. Click an environment or operation to open
its detail, the Machines/Activity buttons to change views, and the
Overview/Definition/Activity buttons to inspect a machine. The connection
fields, Connect/Quit buttons, footer commands and help Close button are also
clickable. The wheel navigates the list or scrolls the detail under the pointer.
Visible hit areas follow list scrolling, filters and terminal resizing.
Mouse capture is released when the TUI exits. Your terminal must forward mouse
events; keyboard navigation remains available.

| Key | Action |
| --- | --- |
| `m`, `a` | Machines, global Activity |
| Up/Down, `k`/`j` | Move through the list; scroll when detail has focus |
| Tab, Left/Right | Switch list/detail focus |
| Enter | Open selected detail |
| `1`, `2`, `3` | Machine overview, saved YAML, activity for this machine |
| `/` | Search current list; Enter applies, Escape cancels |
| Escape | Clear applied search and return focus to the list |
| PageUp/PageDown, Home | Scroll detail, return to top |
| `r` | Refresh now |
| `c` | Reconnect after the current read finishes |
| `?` | Keyboard help |
| `q`, Ctrl-C | Quit |

The connection screen uses Tab/Shift-Tab to switch fields, Ctrl-U to clear a
field, Enter to connect and Escape to quit. The interface supports bracketed
paste. At widths below 90 columns only the focused pane is shown; Tab switches
panes. Minimum size is 40 columns by 12 rows.

## Evidence and failure behavior

Refresh runs every five seconds after the previous read finishes, without
overlapping batches. Each request has a 15-second timeout. Network reads run
outside the terminal loop, so navigation and quitting remain responsive.
Changing selection discards detail replies for the previously selected object.

The lists contain the latest 100 environments and 100 operations returned by
the server. Machine activity filters this recent operation window, not the
entire persistent journal. Search applies to these loaded records. An empty
list is distinct from an error; failed list refreshes retain the previous
records with a visible **STALE** warning. Failed power observations display
unknown rather than a cached running/stopped claim.

`registered` describes a saved association, not a running or ready VM.
Power is a separate instance/scope/node/VMID-checked GET. A saved guest address
is a historical connection hint, never SSH identity or current readiness proof.
Definitions are checked against their SHA-256 hash. Provisioning stages are
checked against the selected operation and environment; their own recorded
status may be newer than the history summary. `uncertain` and `interrupted`
remain explicit. This client only issues GET requests. Closing it cannot
cancel, repeat or submit a server operation.

## Verification and limits

```sh
make check-tui
```

This checks formatting, strict Clippy and Rust tests, builds the real binary,
then runs it in a Unix PTY against a generated localhost HTTPS server. The PTY
check needs Python 3 and OpenSSL and uses temporary files and an ephemeral
certificate/key. It checks connection/authentication, machine/power display,
YAML, activity/stages, refresh, search/help, resize, quitting during an in-flight
request, token non-display and terminal restoration. It uses SGR mouse events
to connect, switch tabs/views, select an operation, refresh, search, open/close
help and quit, and checks mouse-capture restoration. No Proxmox or personal
configuration is used. TLS/authentication failures, cross-target replies and
late replies are also covered by Rust tests.

Runtime acceptance currently covers Linux with generated HTTPS fixtures.
macOS/Windows runtime and use against a real deployed control plane remain
unverified. There is no SSH shortcut, embedded terminal, creation form or VM
mutation in this version. The next bounded extension is launching the system
SSH client from a reviewed guest hint while restoring the TUI afterward.
