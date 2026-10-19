import { useEffect, useRef, useState, type FormEvent } from "react";

/** Android IME input; xterm continues to render the actual remote terminal. */
export function MobileTerminalInput({ disabled, send, disconnect }: { disabled: boolean; send: (bytes: Uint8Array) => void; disconnect: () => void }) {
  const [value, setValue] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const committed = useRef("");
  const composing = useRef(false);
  function reset() { committed.current = ""; composing.current = false; setValue(""); }
  useEffect(() => { if (disabled) reset(); }, [disabled]);
  function commit(next: string) {
    if (disabled) return;
    const before = Array.from(committed.current), after = Array.from(next);
    let common = 0;
    while (common < before.length && common < after.length && before[common] === after[common]) common++;
    const edit = "\x7f".repeat(before.length - common) + after.slice(common).join("");
    if (edit) send(new TextEncoder().encode(edit));
    committed.current = next;
  }
  function key(bytes: number[]) {
    if (disabled) return;
    // A control key may move or change the remote line. Start a new local input
    // segment so a later IME edit cannot erase text outside that segment.
    commit(input.current?.value || value); reset(); send(new Uint8Array(bytes));
    input.current?.focus();
  }
  function enter(event: FormEvent) { event.preventDefault(); key([13]); }
  return <div className="mobile-terminal-controls">
    <form onSubmit={enter}>
      <label>Terminal input
        <input ref={input} type="password" value={value} maxLength={1024} disabled={disabled} enterKeyHint="send" autoComplete="off" autoCapitalize="none" autoCorrect="off" spellCheck={false}
          onCompositionStart={() => { composing.current = true; }}
          onCompositionEnd={event => { composing.current = false; commit(event.currentTarget.value); }}
          onChange={event => { const next = event.currentTarget.value; setValue(next); if (!composing.current) commit(next); }} />
      </label>
      <button type="submit" disabled={disabled}>Enter</button>
    </form>
    <div className="connection-fields">
      <button type="button" disabled={disabled} onClick={() => key([9])}>Tab</button>
      <button type="button" disabled={disabled} onClick={() => key([27])}>Esc</button>
      <button type="button" disabled={disabled} onClick={() => key([127])}>Backspace</button>
      <button type="button" disabled={disabled} onClick={() => key([3])}>Send Ctrl-C</button>
      <button type="button" disabled={disabled} onClick={disconnect}>Disconnect</button>
    </div>
  </div>;
}
