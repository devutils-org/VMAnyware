// The reviewed-operation module: one state machine for every confirmed remote action
// (start/shutdown, snapshot, restore, delete, recreate). Each action supplies an
// adapter with its native commands and evidence checks; this module owns ordering,
// request-key retention, stale-response protection and release of settled requests.

export type OperationStatus = "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain";
export type OperationLike = { id: string; status: OperationStatus };

export interface ReviewedOperationAdapter<Input, Review, Operation extends OperationLike> {
  /** Native read-only review under a fresh request key; throw if it does not match the request. */
  prepare(requestKey: string, input: Input): Promise<Review>;
  /** Native confirmed submission of the reviewed request. */
  submit(requestKey: string, review: Review): Promise<Operation>;
  /** Native read of the retained request's operation. Never resubmits. */
  read(requestKey: string): Promise<Operation>;
  /** Native release of the retained request state. */
  clear(requestKey: string): Promise<void>;
  /** Throw when an operation does not carry evidence matching the review. */
  validate(operation: Operation, review: Review, previous: Operation | null): void;
  /** Definitive outcomes only; uncertain or still-running work is never terminal. */
  isTerminal(operation: Operation): boolean;
  /** Native refusals that happen before any provider write, so the review can be confirmed again. */
  isRefusedBeforeSubmission?(failure: unknown): boolean;
}

export type ReviewedOperationState<Review, Operation> = {
  requestKey: string;
  review: Review | null;
  submitted: boolean;
  operation: Operation | null;
  released: boolean;
  busy: boolean;
  error: string;
};

export const initialState = <R, O>(): ReviewedOperationState<R, O> =>
  ({ requestKey: "", review: null, submitted: false, operation: null, released: false, busy: false, error: "" });

/** True while the retained request must not be replaced by another action. */
export function locked(state: ReviewedOperationState<unknown, unknown>) {
  return !state.released && (state.busy || state.review !== null || state.submitted);
}

export class ReviewedOperation<Input, Review, Operation extends OperationLike> {
  private state: ReviewedOperationState<Review, Operation> = initialState();
  private generation = 0;
  private readonly adapter: ReviewedOperationAdapter<Input, Review, Operation>;
  private readonly onChange: (state: ReviewedOperationState<Review, Operation>) => void;
  private readonly fallbackError: string;
  private readonly newKey: () => string;

  constructor(
    adapter: ReviewedOperationAdapter<Input, Review, Operation>,
    onChange: (state: ReviewedOperationState<Review, Operation>) => void,
    fallbackError: string,
    newKey: () => string = () => crypto.randomUUID(),
  ) {
    this.adapter = adapter; this.onChange = onChange; this.fallbackError = fallbackError; this.newKey = newKey;
  }

  get current() { return this.state; }

  private set(patch: Partial<ReviewedOperationState<Review, Operation>>) {
    this.state = { ...this.state, ...patch };
    this.onChange(this.state);
  }
  private message(failure: unknown) { return typeof failure === "string" ? failure : this.fallbackError; }

  /** Stops applying results of in-flight calls (unmount or replacement). */
  dispose() { this.generation++; }

  async review(input: Input) {
    if (this.state.busy || this.state.submitted || (this.state.review && !this.state.released)) return;
    const current = ++this.generation, key = this.newKey();
    this.set({ ...initialState<Review, Operation>(), requestKey: key, busy: true });
    try {
      const review = await this.adapter.prepare(key, input);
      if (current === this.generation) this.set({ review });
    } catch (failure) { if (current === this.generation) this.set({ error: this.message(failure) }); }
    finally { if (current === this.generation) this.set({ busy: false }); }
  }

  /** Submits the reviewed request, or retries the same key after an unknown outcome. */
  async submit() {
    const { review, requestKey, operation, busy } = this.state;
    if (!review || operation || busy) return;
    const current = ++this.generation;
    this.set({ submitted: true, busy: true, error: "" });
    try {
      const result = await this.adapter.submit(requestKey, review);
      if (current !== this.generation) return;
      this.adapter.validate(result, review, null);
      this.set({ operation: result });
    } catch (failure) {
      if (current !== this.generation) return;
      // Only a refusal proven to precede any write may return to the review.
      this.set({ error: this.message(failure), ...(this.adapter.isRefusedBeforeSubmission?.(failure) ? { submitted: false } : {}) });
    } finally { if (current === this.generation) this.set({ busy: false }); }
  }

  async refresh() {
    const { review, requestKey, operation, busy } = this.state;
    if (!review || !operation || busy) return;
    const current = ++this.generation;
    this.set({ busy: true, error: "" });
    try {
      const result = await this.adapter.read(requestKey);
      if (current !== this.generation) return;
      this.adapter.validate(result, review, operation);
      this.set({ operation: result });
    } catch (failure) { if (current === this.generation) this.set({ error: this.message(failure) }); }
    finally { if (current === this.generation) this.set({ busy: false }); }
  }

  /** Releases a definitively settled request while keeping its result visible. */
  async release() {
    const { operation, requestKey, released } = this.state;
    if (!operation || released || !this.adapter.isTerminal(operation)) return false;
    try { await this.adapter.clear(requestKey); this.set({ released: true }); return true; }
    catch { return false; }
  }

  /** Abandons an unsubmitted review, or explicitly releases a settled request. */
  async cancel() {
    const { requestKey, submitted, operation, busy } = this.state;
    if (busy || (submitted && !(operation && this.adapter.isTerminal(operation)))) return false;
    const current = ++this.generation;
    this.set({ busy: true, error: "" });
    try {
      if (requestKey) await this.adapter.clear(requestKey);
      if (current !== this.generation) return false;
      this.set(initialState());
      return true;
    } catch (failure) { if (current === this.generation) this.set({ busy: false, error: this.message(failure) }); return false; }
  }

  terminal() { return !!this.state.operation && this.adapter.isTerminal(this.state.operation); }
}
