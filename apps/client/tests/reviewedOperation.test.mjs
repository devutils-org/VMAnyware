import { test } from "node:test";
import assert from "node:assert/strict";
import { ReviewedOperation, locked } from "../src/reviewedOperation.ts";

// An in-memory adapter that records native calls and returns scripted results.
function adapter(script = {}) {
  const calls = [];
  const statuses = script.statuses ?? ["running", "succeeded"];
  return {
    calls,
    async prepare(key, input) { calls.push(["prepare", key, input]); if (script.prepareFails) throw "review refused"; return { target: input }; },
    async submit(key) { calls.push(["submit", key]); if (script.submitFails) throw script.submitFails; return { id: "op", status: "queued" }; },
    async read(key) { calls.push(["read", key]); return { id: "op", status: statuses.shift() ?? "succeeded" }; },
    async clear(key) { calls.push(["clear", key]); },
    validate(operation, review, previous) { if (operation.id !== "op" || (previous && previous.id !== operation.id)) throw "mismatch"; },
    isTerminal(operation) { return operation.status === "succeeded" || operation.status === "failed"; },
    isRefusedBeforeSubmission: failure => failure === "refused before POST",
  };
}
const keys = () => { let n = 0; return () => `key-${++n}`; };

test("review, submit, follow and release one retained request key", async () => {
  const native = adapter();
  const flow = new ReviewedOperation(native, () => {}, "fallback", keys());
  await flow.review("vm-9001");
  assert.ok(locked(flow.current));
  await flow.submit();
  assert.equal(flow.current.operation.status, "queued");
  assert.equal(await flow.release(), false, "queued work is never released");
  await flow.refresh();
  assert.equal(flow.current.operation.status, "running");
  await flow.refresh();
  assert.ok(flow.terminal());
  assert.equal(await flow.release(), true);
  assert.equal(locked(flow.current), false);
  assert.equal(flow.current.operation.status, "succeeded", "the result stays visible");
  assert.deepEqual(native.calls.map(([name, key]) => [name, key]),
    [["prepare", "key-1"], ["submit", "key-1"], ["read", "key-1"], ["read", "key-1"], ["clear", "key-1"]]);
});

test("uncertain work stays locked and is never released", async () => {
  const flow = new ReviewedOperation(adapter({ statuses: ["uncertain"] }), () => {}, "fallback", keys());
  await flow.review("vm");
  await flow.submit();
  await flow.refresh();
  assert.equal(flow.current.operation.status, "uncertain");
  assert.equal(await flow.release(), false);
  assert.equal(await flow.cancel(), false, "cannot abandon a submitted, unsettled request");
  assert.ok(locked(flow.current));
});

test("an unknown submission outcome keeps the same key for retry; a proven refusal returns to review", async () => {
  const unknown = adapter({ submitFails: "connection lost" });
  const flow = new ReviewedOperation(unknown, () => {}, "fallback", keys());
  await flow.review("vm");
  await flow.submit();
  assert.equal(flow.current.submitted, true);
  assert.equal(flow.current.error, "connection lost");
  unknown.submit = async key => { unknown.calls.push(["submit", key]); return { id: "op", status: "queued" }; };
  await flow.submit();
  assert.deepEqual(unknown.calls.filter(([name]) => name === "submit").map(([, key]) => key), ["key-1", "key-1"]);

  const refused = new ReviewedOperation(adapter({ submitFails: "refused before POST" }), () => {}, "fallback", keys());
  await refused.review("vm");
  await refused.submit();
  assert.equal(refused.current.submitted, false);
});

test("stale responses after dispose are ignored, and a cancelled review is cleared", async () => {
  const native = adapter();
  const states = [];
  const flow = new ReviewedOperation(native, state => states.push(state), "fallback", keys());
  const pending = flow.review("vm");
  flow.dispose();
  await pending;
  assert.equal(flow.current.review, null, "a disposed review never lands");
  // A disposed controller belongs to an unmounted view; a new one takes over.
  const next = new ReviewedOperation(native, () => {}, "fallback", () => "key-2");
  await next.review("vm");
  assert.equal(await next.cancel(), true);
  assert.deepEqual(native.calls.at(-1), ["clear", "key-2"]);
  assert.equal(locked(next.current), false);
});

test("evidence that does not match the review is reported, not accepted", async () => {
  const native = adapter();
  native.read = async () => ({ id: "other", status: "succeeded" });
  const flow = new ReviewedOperation(native, () => {}, "fallback", keys());
  await flow.review("vm");
  await flow.submit();
  await flow.refresh();
  assert.equal(flow.current.operation.status, "queued");
  assert.equal(flow.current.error, "mismatch");
});
