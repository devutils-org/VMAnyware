import { test } from "node:test";
import assert from "node:assert/strict";
import { observeInstance, validateLifecyclePreflight } from "../src/instanceObservation.ts";

const instance = { id: "a".repeat(32), definitionName: "example", definitionHash: "b".repeat(64), definitionRevision: 1,
  providerReference: { provider: "proxmox", scopeId: "c".repeat(64), node: "pve-test", vmid: 202 }, status: "registered", createdAt: "2026-10-18" };
function operation(status = "queued") {
  return { id: "d".repeat(32), kind: "instance_observe", instanceId: instance.id, status, result: status === "succeeded" ? {
    instanceId: instance.id, providerScopeId: instance.providerReference.scopeId,
    observation: { node: "pve-test", vmid: 202, status: "running", qmpStatus: "paused" },
  } : null, failure: status === "failed" ? "provider_unavailable" : null, updatedAt: "2026-10-18" };
}
function cache() {
  const keys = new Map();
  return { keys, loadKey: key => keys.get(key) ?? null, saveKey: (key, value) => value === null ? keys.delete(key) : keys.set(key, value), wait: async () => {} };
}
test("observation polls one bound operation and clears its key only after terminal state", async () => {
  const options = cache(), calls = [], updates = [];
  const responses = [operation(), operation("running"), operation("succeeded")];
  assert.equal(await observeInstance(8080, instance, async (command, args) => { calls.push({ command, args }); return responses.shift(); }, value => updates.push(value.status), () => true, options), "complete");
  assert.deepEqual(updates, ["queued", "running", "succeeded"]);
  assert.equal(calls[0].command, "begin_instance_refresh");
  for (const call of calls.slice(1)) {
    assert.equal(call.command, "read_instance_operation");
    assert.equal(call.args.instanceId, instance.id);
    assert.equal(call.args.id, operation().id);
  }
  assert.equal(options.keys.size, 0);
});
test("a monitoring deadline pauses and reuses the same key on explicit resume", async () => {
  const options = cache(); let time = 0, requestKey;
  const first = await observeInstance(8080, instance, async (_, args) => { requestKey = args.requestKey; time = 60000; return operation(); }, () => {}, () => true, { ...options, now: () => time });
  assert.equal(first, "paused"); assert.equal(options.keys.size, 1);
  assert.equal(await observeInstance(8080, instance, async (_, args) => { assert.equal(args.requestKey, requestKey); return operation("succeeded"); }, () => {}, () => true, options), "complete");
  assert.equal(options.keys.size, 0);
});
test("a late response after leaving the view neither updates it nor clears its resume key", async () => {
  const options = cache(); let active = true;
  const result = await observeInstance(8080, instance, async () => { active = false; return operation("succeeded"); }, () => assert.fail("stale update"), () => active, options);
  assert.equal(result, "stale"); assert.equal(options.keys.size, 1);
});
test("leaving during the poll delay makes no additional native request", async () => {
  const options = cache(); let active = true, calls = 0;
  const result = await observeInstance(8080, instance, async () => { calls++; return operation(); }, () => {}, () => active, { ...options, wait: async () => { active = false; } });
  assert.equal(result, "stale"); assert.equal(calls, 1);
});
test("wrong scope, target, kind or operation identity never displays a result", async () => {
  for (const change of [
    value => { value.instanceId = "f".repeat(32); },
    value => { value.kind = "inventory_refresh"; },
    value => { value.result.providerScopeId = "f".repeat(64); },
    value => { value.result.observation.vmid = 999; },
    value => { value.result.observation.node = "other"; },
  ]) {
    const bad = operation("succeeded"); change(bad);
    await assert.rejects(observeInstance(8080, instance, async () => bad, () => assert.fail("unrelated result"), () => true, cache()), error => typeof error === "string" && error.includes("unrelated"));
  }
  let calls = 0;
  await assert.rejects(observeInstance(8080, instance, async () => {
    if (++calls === 1) return operation();
    const other = operation("succeeded"); other.id = "f".repeat(32); return other;
  }, value => assert.equal(value.status, "queued"), () => true, cache()));
});
test("network failure retains the key and terminal interruption does not retry itself", async () => {
  const options = cache(); let calls = 0;
  await assert.rejects(observeInstance(8080, instance, async () => {
    if (++calls === 1) return operation();
    throw new Error("transport failed");
  }, () => {}, () => true, options));
  assert.equal(options.keys.size, 1);
  const interrupted = operation("interrupted"); interrupted.failure = "process_interrupted";
  const states = [];
  await observeInstance(8080, instance, async () => { calls++; return interrupted; }, value => states.push(value.status), () => true, options);
  assert.deepEqual(states, ["interrupted"]); assert.equal(calls, 3);
  assert.equal(options.keys.size, 0);
});

function lifecycleReport() {
  return { instanceId: instance.id, providerScopeId: instance.providerReference.scopeId,
    definitionHash: instance.definitionHash, expectedMarker: `vmanyware-${instance.id}`,
    configDigest: "a".repeat(40), observation: { node: "pve-test", vmid: 202, status: "stopped", qmpStatus: null },
    checks: ["instance_marker", "not_template", "unlocked", "no_pending_changes", "configuration_unchanged"].map(code => ({ code, status: "passed", message: "Fixed check" })),
    startPreconditions: "passed", shutdownPreconditions: "failed", executionEnabled: false };
}
test("lifecycle reports preserve explicit unknown and execution-disabled states", () => {
  const report = lifecycleReport();
  assert.doesNotThrow(() => validateLifecyclePreflight(report, instance));
  report.startPreconditions = "failed"; report.shutdownPreconditions = "unknown";
  report.observation.status = "running";
  assert.doesNotThrow(() => validateLifecyclePreflight(report, instance));
  assert.equal(report.executionEnabled, false);
});
test("unrelated, incomplete or malformed capability lifecycle reports are rejected", () => {
  for (const change of [
    report => { report.instanceId = "f".repeat(32); },
    report => { report.providerScopeId = "f".repeat(64); },
    report => { report.definitionHash = "f".repeat(64); },
    report => { report.expectedMarker = "another-instance"; },
    report => { report.configDigest = "not-a-digest"; },
    report => { report.observation.node = "other"; },
    report => { report.observation.vmid = 203; },
    report => { report.executionEnabled = "true"; },
    report => { report.checks.pop(); },
    report => { report.checks[0].code = report.checks[1].code; },
    report => { report.startPreconditions = "future-state"; },
    report => { report.observation.status = "ready"; },
  ]) {
    const report = lifecycleReport(); change(report);
    assert.throws(() => validateLifecyclePreflight(report, instance), error => typeof error === "string" && error.includes("invalid lifecycle"));
  }
});

test("enabled capability is advisory and does not replace confirmed action inputs", () => {
  const report = lifecycleReport(); report.executionEnabled = true;
  validateLifecyclePreflight(report, instance);
});
