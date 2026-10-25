import { lazy, Suspense } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Session } from "./Console";
import { Icon } from "./icons";
import { shortcutLabel } from "./shortcuts";
const SshTerminal = lazy(async () => ({ default: (await import("./SshTerminal")).SshTerminal }));

/** Open terminal sessions as tabs. Owns native session lifetime: closing a tab closes it. */
export function Sessions({ sessions, active, onSelect, onClosed, maximized, onToggleMaximize }: { sessions: Session[]; active: string | null; onSelect: (id: string) => void; onClosed: (id: string) => void; maximized: boolean; onToggleMaximize: () => void }) {
  function close(id: string) {
    void invoke("close_ssh_session", { sessionId: id }).catch(() => {});
    onClosed(id);
  }
  if (sessions.length === 0) return null;
  return <div className="sessions">
    <div className="session-tabs" role="tablist">
      {sessions.map(session => <div key={session.id} className="session-tab" role="tab" aria-selected={session.id === active}>
        <button type="button" className="session-title" onClick={() => onSelect(session.id)}><Icon name="terminal" />{session.title}</button>
        <button type="button" className="icon" aria-label={`Close ${session.title}`} title={`Close session (${shortcutLabel("closeSession")})`} onClick={() => close(session.id)}><Icon name="close" size={14} /></button>
      </div>)}
      <button type="button" className="icon session-maximize" aria-label={maximized ? "Exit full window" : "Full window"} title={maximized ? `Exit full window (${shortcutLabel("fullWindow")} or ⇧Esc)` : `Full window (${shortcutLabel("fullWindow")})`} onClick={onToggleMaximize}><Icon name={maximized ? "minimize" : "maximize"} /></button>
    </div>
    {/* All sessions stay mounted so switching tabs keeps their streams. */}
    {sessions.map(session => <div key={session.id} hidden={session.id !== active} className="session-panel">
      <Suspense fallback={<p role="status">Preparing terminal…</p>}><SshTerminal id={session.id} account={session.account} /></Suspense>
    </div>)}
  </div>;
}
