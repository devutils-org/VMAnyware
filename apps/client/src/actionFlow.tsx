import { useEffect, useRef, useState } from "react";
import { initialState, locked, ReviewedOperation, type OperationLike, type ReviewedOperationAdapter } from "./reviewedOperation";

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
    <ul className="steps">{checks.map(check => <li key={check.code} className={`step ${check.status}`}><span className="step-icon" aria-hidden="true" /><span className="step-label">{labels[check.code] || "Unrecognized prerequisite"}</span><span className={`chip state ${check.status}`}>{check.status}</span></li>)}</ul>
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

/**
 * React binding of the reviewed-operation module: automatic review on open, bounded
 * polling, automatic release of settled requests and lock reporting to the shell.
 */
export function useReviewedOperation<Input, Review, Operation extends OperationLike>(
  adapter: ReviewedOperationAdapter<Input, Review, Operation>,
  options: { fallbackError: string; disabled: boolean; autoReview?: Input; onLock: (locked: boolean) => void; onSettled?: (succeeded: boolean) => void; onDone?: () => void },
) {
  const latest = useRef(adapter);
  latest.current = adapter;
  const [state, setState] = useState(initialState<Review, Operation>);
  const controller = useRef<ReviewedOperation<Input, Review, Operation> | null>(null);
  if (!controller.current) {
    // Delegate to the latest adapter so commands always see current props.
    const delegate: ReviewedOperationAdapter<Input, Review, Operation> = {
      prepare: (key, input) => latest.current.prepare(key, input),
      submit: (key, review) => latest.current.submit(key, review),
      read: key => latest.current.read(key),
      clear: key => latest.current.clear(key),
      validate: (operation, review, previous) => latest.current.validate(operation, review, previous),
      isTerminal: operation => latest.current.isTerminal(operation),
      isRefusedBeforeSubmission: failure => latest.current.isRefusedBeforeSubmission?.(failure) ?? false,
    };
    controller.current = new ReviewedOperation(delegate, setState, options.fallbackError);
  }
  const flow = controller.current;
  const { onLock } = options;
  useEffect(() => { onLock(locked(state)); }, [state, onLock]);
  useEffect(() => () => { flow.dispose(); onLock(false); }, [flow, onLock]);
  useAutoRun(options.autoReview !== undefined, () => void flow.review(options.autoReview as Input), []);
  const terminal = !!state.operation && adapter.isTerminal(state.operation);
  useOperationPolling(!!state.operation && !terminal && !options.disabled, state.operation, () => void flow.refresh());
  useAutoRun(terminal && !state.released, () => {
    void flow.release().then(released => { if (released) options.onSettled?.(state.operation?.status === "succeeded"); });
  }, [state.operation?.status]);
  return {
    state, terminal, pending: !!state.operation && !terminal,
    review: (input: Input) => void flow.review(input),
    submit: () => void flow.submit(),
    /** Cancel an unsubmitted review, or release a settled result, then close. */
    close: () => void flow.cancel().then(done => { if (done) options.onDone?.(); }),
  };
}
