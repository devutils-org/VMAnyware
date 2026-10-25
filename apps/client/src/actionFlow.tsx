import { useEffect, useRef } from "react";

/** Re-reads a submitted operation until it settles. Reads only; never resubmits. */
export function useOperationPolling(active: boolean, operation: unknown, refresh: () => void) {
  const latest = useRef(refresh), count = useRef(0);
  latest.current = refresh;
  useEffect(() => {
    if (!active) { count.current = 0; return; }
    // Bounded: fast at first, then slower, and stop after about fifteen minutes.
    if (count.current >= 240) return;
    const timer = window.setTimeout(() => { count.current++; latest.current(); }, count.current < 10 ? 1500 : 4000);
    return () => window.clearTimeout(timer);
  }, [active, operation]);
}

/** Collapsed prerequisite list; expanded when any check did not pass. */
export function CheckSummary({ checks, labels }: { checks: { code: string; status: string }[]; labels: Record<string, string> }) {
  const passed = checks.filter(check => check.status === "passed").length;
  return <details className="checks" open={passed !== checks.length || undefined}>
    <summary><span className={passed === checks.length ? "ok" : "warn"}>{passed}/{checks.length}</span> prerequisite checks passed</summary>
    <ul>{checks.map(check => <li key={check.code} className={check.status}>{labels[check.code] || "Unrecognized prerequisite"}: {check.status}</li>)}</ul>
  </details>;
}

/** Runs a read once per dependency change, after mount settles. Deferring past the
 *  development double-mount avoids a second concurrent native request ("busy"). */
export function useAutoRun(enabled: boolean, run: () => void, deps: unknown[]) {
  const latest = useRef(run);
  latest.current = run;
  useEffect(() => {
    if (!enabled) return;
    const timer = window.setTimeout(() => latest.current(), 0);
    return () => window.clearTimeout(timer);
  }, [enabled, ...deps]);
}
