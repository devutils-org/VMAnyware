import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { TerminalSession } from "./terminalSession";
import "@xterm/xterm/css/xterm.css";

export function SshTerminal({ id, account }: { id: string; account: string }) {
  const container = useRef<HTMLDivElement>(null);
  const transport = useRef<TerminalSession | null>(null);
  const emulator = useRef<Terminal | null>(null);
  const [status, setStatus] = useState("Connected");
  const [error, setError] = useState("");
  const [closed, setClosed] = useState(false);
  useEffect(() => {
    if (!container.current) return;
    let active = true;
    setStatus("Connected"); setError(""); setClosed(false);
    const term = new Terminal({
      cols: 80, rows: 24, scrollback: 1000, fontSize: 13,
      theme: { background: "#101820", foreground: "#e4eaf1" },
      allowProposedApi: false, allowTransparency: false,
      linkHandler: { activate: () => {} },
    });
    const fit = new FitAddon();
    emulator.current = term;
    term.loadAddon(fit);
    term.open(container.current);
    // Never honor remote clipboard, window title, link navigation or HTML.
    term.parser.registerOscHandler(52, () => true);
    const session = new TerminalSession(id, invoke,
      bytes => new Promise<void>(resolve => {
        if (!active) { resolve(); return; }
        term.write(bytes, resolve);
      }),
      exit => { if (active) setStatus(`Remote process exited with status ${exit}`); },
      () => { if (active) { setStatus("Connection closed"); setClosed(true); term.options.disableStdin = true; } },
      message => { if (active) { setError(message); setClosed(true); term.options.disableStdin = true; } },
    );
    transport.current = session;
    const text = term.onData(data => session.write(new TextEncoder().encode(data)));
    const binary = term.onBinary(data => session.write(Uint8Array.from(data, char => char.charCodeAt(0) & 255)));
    let timer: number | undefined;
    const resize = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        if (!active) return;
        const proposed = fit.proposeDimensions();
        if (proposed && proposed.cols > 0 && proposed.rows > 0) {
          const cols = Math.min(500, proposed.cols), rows = Math.min(500, proposed.rows);
          term.resize(cols, rows);
          session.resize(cols, rows);
        }
      }, 100);
    };
    const observer = new ResizeObserver(resize);
    observer.observe(container.current);
    // StrictMode's discarded first effect must never issue a native read/close.
    queueMicrotask(() => { if (active) { session.start(); resize(); term.focus(); } });
    return () => {
      active = false;
      session.stop(false);
      if (transport.current === session) transport.current = null;
      if (emulator.current === term) emulator.current = null;
      observer.disconnect(); window.clearTimeout(timer);
      text.dispose(); binary.dispose(); term.dispose();
      // SshAuthentication owns native lifetime, including a pending lazy mount.
      // Disposing this renderer must not close a surviving StrictMode session.
    };
  }, [id]);
  function disconnect() {
    transport.current?.stop();
    if (emulator.current) emulator.current.options.disableStdin = true;
    setStatus("Disconnected"); setClosed(true);
  }
  return <div>
    <h3>SSH terminal · {account}</h3>
    <p role="status">{status}</p>
    {error && <p role="alert" className="error">{error}</p>}
    <div ref={container} className="ssh-terminal" aria-label={`SSH terminal for ${account}`} />
    <div className="connection-fields">
      <button type="button" disabled={closed} onClick={() => transport.current?.write(new Uint8Array([3]))}>Send Ctrl-C</button>
      <button type="button" disabled={closed} onClick={disconnect}>Disconnect</button>
    </div>
    {closed && <p>List identities again to open a fresh connection.</p>}
  </div>;
}
