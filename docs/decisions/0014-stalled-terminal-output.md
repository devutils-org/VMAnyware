# ADR 0014 — Fail visibly when native terminal output remains unread

Date: 2026-10-17
Status: Implemented after a failing real-shell pressure test.

The native real-shell fixture exposed a limit in the pinned russh 0.64.1 path.
Its client encrypted CHANNEL_DATA handler awaits delivery to the channel queue
before continuing the connection loop. If the UI stops pulling, bounded queues
can fill and prevent that loop from processing queued Ctrl-C input. A successful
native write means queued bytes, not delivery; it cannot prove interruption.
Increasing queue sizes would postpone this condition without resolving it.

Keep bounded buffering and preserve output rather than silently discarding it.
When an output event cannot be forwarded for three seconds, mark the session
failed and close its owned socket. Any blocking write also uses that earlier
stall deadline. Retain failure state outside the full output queue: queued output
drains, then the reader observes Failed once, then Closed. Do not report normal
closure merely because a failure event could not fit in the queue.

A paused-reader real /bin/sh/yes test must close the owned shell/connection within
five seconds, drain a finite backlog and observe failure. A freshly selected
connection must still interrupt yes within two seconds while output is actively
consumed and remain usable. The pre-fix paused-reader Ctrl-C test failed; this
policy deliberately closes instead of claiming that interruption succeeded.

This is a prototype limitation. A renderer suspended/backgrounded for long enough
can lose its connection. Robust indefinite output pause/resume requires an SSH
transport with receive-window control that preserves outgoing control handling,
or a reviewed upstream/library change. No local dependency fork or unbounded
spool is introduced for this slice. No real infrastructure or personal agent is
needed for the evidence.

Primary implementation inspected: pinned russh client/encrypted.rs CHANNEL_DATA
and CHANNEL_EXTENDED_DATA handlers, and session.rs adjust_window_size. Local
fixture failure and passing policy tests are recorded in STATE.md.
