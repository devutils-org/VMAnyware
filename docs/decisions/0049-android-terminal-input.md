# 0049 — Android terminal input through a standard WebView field

Status: implemented; owned Android native input/SSH smoke passes, full IME acceptance pending.

## Context

The owned Android SSH experiment observed duplicated or missing characters when
Android key input used xterm's hidden textarea. A protocol-only SSH pass did not
prove usable mobile typing. Keep xterm for rendering and the existing native SSH
session for transport; do not add another SSH implementation or runtime package.

## Decision

On Android, disable xterm user stdin and display a standard controlled WebView
input. Incremental edits send UTF-8 suffixes and DEL for replaced Unicode
codepoints. Composition is withheld until its commit event. Special keys flush
and reset the local input segment before sending their terminal control bytes,
so later local edits cannot erase the prior segment after remote completion.
Desktop input continues through xterm.

The field uses password rendering and disables suggestions/autocorrection: local
input must not expose text when the remote application disables terminal echo.
Enter, Tab, Escape, Backspace, interrupt and disconnect are explicit controls.
Native Activity padding reserves keyboard/system-bar/cutout insets and forwards
zero values for handled types to avoid applying them twice. Native viewport
behavior requires a rebuilt APK test; browser layout is insufficient evidence.

## Evidence and limits

The opt-in Firefox simulated-IPC check exercises the actual React input component
and asserts exact bytes for replacement, Unicode deletion, composition committed
once, and special-key boundaries. It does not prove Android WebView event order,
software IME behavior, native SSH delivery or a usable full-screen terminal.
The owned native SSH harness must establish those runtime boundaries separately.

Codepoint deletion does not establish grapheme-cluster editing for every remote
terminal/application. The input field is a streaming segment, not an authoritative
copy of the remote command line. Full-screen editors, cursor movement, selection,
paste, external keyboards, accessibility and physical-device keyboards remain
acceptance work; do not claim general terminal parity from fixed shell commands.
No new secret persistence or key export is introduced.


The 2026-10-19 rebuilt x86_64 APK passes the owned Android 15 native harness:
strict trust, actual OS shell input, rotation/stty, Ctrl-C, reconnect and process
death cleanup. An explicit ADB reverse tunnel was necessary because a separate
host-alias TCP probe returned network unreachable. This proves the native
input/SSH path without proving direct networking or complete software-IME support.
Narrow-screen controls wrap with 44px touch targets; the actual screenshot was
reviewed. Native IME inset handling compiles and is exercised with a visible
keyboard, but other WebView/physical-device parity remains open.
