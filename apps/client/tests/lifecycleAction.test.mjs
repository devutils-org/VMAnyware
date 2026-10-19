import { test } from "node:test";
import assert from "node:assert/strict";
import { confirmedLifecycle, monitorLifecycle, validatePendingLifecycle } from "../src/lifecycleAction.ts";
const instance = { id: "a".repeat(32), definitionHash: "b".repeat(64), providerReference: { scopeId: "c".repeat(64), node: "pve-test", vmid: 202 } };
function report() {
  return { instanceId: instance.id, definitionHash: instance.definitionHash, providerScopeId: instance.providerReference.scopeId,
    expectedMarker: `vmanyware-${instance.id}`, configDigest: "d".repeat(40), executionEnabled: true,
    observation: { node: "pve-test", vmid: 202, status: "stopped", qmpStatus: null }, startPreconditions: "passed", shutdownPreconditions: "failed",
    checks: ["instance_marker", "not_template", "unlocked", "no_pending_changes", "configuration_unchanged"].map(code => ({ code, status: "passed", message: "fixed" })) };
}
const pending = () => confirmedLifecycle(8080, instance, report(), "start");
function operation(status = "queued") {
  return { id: "e".repeat(32), kind: "vm_start", instanceId: instance.id, status, providerTaskKnown: status === "succeeded",
    failure: status === "uncertain" ? "provider_submission_unconfirmed" : status === "failed" ? "lifecycle_disabled" : null,
    result: status === "succeeded" ? { action: "start", instanceId: instance.id, providerScopeId: instance.providerReference.scopeId, taskStatus: "succeeded",
      observation: { node: "pve-test", vmid: 202, status: "running", qmpStatus: "prelaunch" } } : null, updatedAt: "2026-10-18" };
}
function memory() {
  const saved = [];
  return { saved, persist: value => saved.push(structuredClone(value)), wait: async () => {} };
}
test("confirmation binds target and accepts only enabled passing action conditions", () => {
  const request = pending(); validatePendingLifecycle(request, instance);
  assert.equal(request.approval.confirmed, true);
  assert.equal(request.approval.configDigest, report().configDigest);
  for (const edit of [r => {r.executionEnabled = false;}, r => {r.startPreconditions = "unknown";},
    r => {r.checks[0].status = "failed";}, r => {r.observation.status = "running";}, r => {r.observation.qmpStatus = "paused";}]) {
    const r = report(); edit(r); assert.throws(() => confirmedLifecycle(8080, instance, r, "start"));
  }
  const r = report(); r.observation.status = "running"; r.observation.qmpStatus = "running"; r.shutdownPreconditions = "passed";
  assert.equal(confirmedLifecycle(8080, instance, r, "shutdown").approval.action, "shutdown");
  assert.throws(() => validatePendingLifecycle(request, { ...instance, definitionHash: "f".repeat(64) }));
});
test("monitor persists before POST, then polls one bound operation without claiming guest readiness", async () => {
  const request = pending(), cache = memory(), calls = [], updates = [];
  const replies = [operation(), operation("running"), operation("succeeded")];
  assert.equal(await monitorLifecycle(request, async (command, args) => {
    assert.ok(cache.saved.length); calls.push({command, args}); return replies.shift();
  }, op => updates.push(op.status), () => true, cache), "complete");
  assert.deepEqual(updates, ["queued", "running", "succeeded"]);
  assert.equal(calls[0].command, "begin_lifecycle_action");
  assert.equal(calls[0].args.requestKey, request.requestKey);
  assert.deepEqual(calls[0].args.approval, request.approval);
  assert.ok(calls.slice(1).every(call => call.command === "read_lifecycle_operation" && call.args.id === operation().id));
  assert.equal(cache.saved.at(-1).operationId, operation().id);
});
test("lost submission reply retains exactly the original confirmation and key for explicit resume", async () => {
  const request = pending(), cache = memory();
  await assert.rejects(monitorLifecycle(request, async () => { throw Error("offline"); }, () => {}, () => true, cache));
  const retained = cache.saved.at(-1); assert.deepEqual(retained, request);
  await monitorLifecycle(retained, async (command, args) => {
    assert.equal(command, "begin_lifecycle_action"); assert.equal(args.requestKey, request.requestKey);
    assert.deepEqual(args.approval, request.approval); return operation("failed");
  }, () => {}, () => true, cache);
});
test("known operation resumes by GET after transport failure and after an uncertain result", async () => {
  const request = { ...pending(), operationId: operation().id }, cache = memory();
  await assert.rejects(monitorLifecycle(request, async command => { assert.equal(command, "read_lifecycle_operation"); throw Error("offline"); }, () => {}, () => true, cache));
  assert.equal(await monitorLifecycle(cache.saved.at(-1), async command => { assert.equal(command, "read_lifecycle_operation"); return operation("uncertain"); }, () => {}, () => true, cache), "uncertain");
  assert.equal(cache.saved.at(-1).operationId, request.operationId);
  assert.equal(await monitorLifecycle(cache.saved.at(-1), async command => { assert.equal(command, "read_lifecycle_operation"); return operation("succeeded"); }, () => {}, () => true, cache), "complete");
});
test("deadline pauses without dropping intent and leaving during delay prevents another read", async () => {
  const cache = memory(); let now = 0;
  assert.equal(await monitorLifecycle(pending(), async () => {now = 60000; return operation();}, () => {}, () => true, {...cache, now: () => now}), "paused");
  assert.equal(cache.saved.at(-1).operationId, operation().id);
  let active = true, calls = 0;
  assert.equal(await monitorLifecycle(pending(), async () => {calls++; return operation();}, () => {}, () => active, {...cache, wait: async () => {active = false;}}), "stale");
  assert.equal(calls, 1);
});
test("late acknowledgement retains its ID for GET resumption but cannot update a departed view", async () => {
  const cache = memory(); let active = true, updates = 0;
  assert.equal(await monitorLifecycle(pending(), async () => {active = false; return operation();}, () => {updates++;}, () => active, cache), "stale");
  assert.equal(updates, 0); assert.equal(cache.saved.at(-1).operationId, operation().id);
});
test("persistence failure blocks dispatch and malformed evidence cannot replace saved operation identity", async () => {
  let calls = 0;
  await assert.rejects(monitorLifecycle(pending(), async () => {calls++; return operation();}, () => {}, () => true, {persist: () => {throw Error("storage blocked");}}));
  assert.equal(calls, 0);
  for (const change of [op => {op.id = "bad";}, op => {op.instanceId = "f".repeat(32);}, op => {op.kind = "vm_shutdown";},
    op => {op.providerTaskKnown = false;}, op => {op.result.providerScopeId = "f".repeat(64);},
    op => {op.result.taskStatus = "running";}, op => {op.result.observation.vmid = 203;}, op => {op.result.observation.status = "stopped";}]) {
    const cache = memory(), op = operation("succeeded"); change(op);
    await assert.rejects(monitorLifecycle(pending(), async () => op, () => assert.fail("invalid update"), () => true, cache));
    assert.equal(cache.saved.length, 1); assert.equal(cache.saved[0].operationId, undefined);
  }
});
