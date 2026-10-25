import { test } from "node:test";
import assert from "node:assert/strict";
import { confirmedCreation, monitorCreation, validatePendingCreation, validateCreationOperation } from "../src/creationRequest.ts";
const plan = () => ({ definitionName: "home-smoke", definitionHash: "a".repeat(64), planHash: "b".repeat(64), providerScopeId: "c".repeat(64),
  source: { templateVmid: 9000, templateVersion: "fixture-v1" }, target: { node: "pve-test", vmid: 9001, storage: "local-lvm", bridge: "vmbr0", guestUser: "debian", resources: { cpu: 2, memoryMiB: 2048, diskGiB: 12 } },
  creationEnabled: true, sshKeyCount: 1, packages: [], snapshotAfterProvisioning: false });
const preflight = () => ({ planHash: "b".repeat(64), creationEnabled: true, checks: ["template", "source_disk", "cloud_init_drive", "storage", "storage_capacity", "bridge", "target_vmid"].map(code => ({ code, status: "passed" })) });
const pending = () => confirmedCreation(8080, plan(), preflight(), true);
const queued = () => ({ id: "d".repeat(32), kind: "vm_provision", instanceId: "e".repeat(32), status: "queued", failure: null, result: null });
const ready = () => ({ ...queued(), status: "succeeded", result: { instanceId: "e".repeat(32), providerScopeId: "c".repeat(64), planHash: "b".repeat(64), guestReady: true, guest: { ipv4: "10.10.10.200", cloudInitExitCode: 0, cloudInitClean: true } } });

test("creation requires explicit confirmation, enabled capability and exact successful metadata checks", () => {
  assert.throws(() => confirmedCreation(8080, plan(), preflight(), false));
  for (const variant of ["disabled", "changed", "failed", "missing"]) {
    const report = preflight();
    if (variant === "disabled") report.creationEnabled = false;
    if (variant === "changed") report.planHash = "f".repeat(64);
    if (variant === "failed") report.checks[0].status = "failed";
    if (variant === "missing") report.checks.pop();
    assert.throws(() => confirmedCreation(8080, plan(), report, true));
  }
  const source = plan(), request = confirmedCreation(8080, source, preflight(), true);
  source.target.vmid = 9002;
  assert.equal(request.plan.target.vmid, 9001);
  request.plan.providerScopeId = "invalid";
  assert.throws(() => validatePendingCreation(request));
});


test("lost accepted response retains the identical request key and plan, then polls only the known operation", async () => {
  const request = pending(); let saved, firstArgs; const calls = [];
  await assert.rejects(monitorCreation(request, async (command, args) => {
    firstArgs = structuredClone(args); throw Error("lost reply");
  }, () => assert.fail("No response"), () => true, { persist: p => { saved = p; } }));
  assert.equal(saved.requestKey, request.requestKey);
  assert.equal(saved.operationId, undefined);
  const result = await monitorCreation(saved, async (command, args) => {
    calls.push(command);
    if (calls.length === 1) { assert.deepEqual(args, firstArgs); return queued(); }
    assert.equal(args.id, queued().id); return ready();
  }, () => {}, () => true, { persist: p => { saved = p; }, wait: async () => {} });
  assert.equal(result, "complete");
  assert.deepEqual(calls, ["begin_local_creation", "read_local_creation"]);
  assert.equal(saved.operationId, queued().id);
  assert.equal(saved.instanceId, queued().instanceId);
});

test("persistence failure prevents submission and stale results still retain the known operation", async () => {
  let calls = 0;
  await assert.rejects(monitorCreation(pending(), async () => { calls++; }, () => {}, () => true,
    { persist: () => { throw Error("Storage unavailable"); } }));
  assert.equal(calls, 0);
  let active = true, saved;
  const result = await monitorCreation(pending(), async () => { active = false; return queued(); },
    () => assert.fail("stale render"), () => active, { persist: p => { saved = p; } });
  assert.equal(result, "stale"); assert.equal(saved.operationId, queued().id);
});

test("monitoring timeout and restart preserve operation identity and issue only GET-style native reads", async () => {
  let time = 0, saved;
  assert.equal(await monitorCreation(pending(), async () => { time = 60001; return queued(); }, () => {}, () => true,
    { now: () => time, persist: p => { saved = p; } }), "paused");
  const reloaded = JSON.parse(JSON.stringify(saved));
  assert.equal(await monitorCreation(reloaded, async (command, args) => {
    assert.equal(command, "read_local_creation"); assert.equal(args.id, queued().id); return ready();
  }, () => {}, () => true, { persist: p => { saved = p; } }), "complete");
});

test("unrelated operation, scope, plan, instance and false readiness never display success", () => {
  const request = pending(); request.operationId = queued().id; request.instanceId = queued().instanceId;
  for (const change of [o => { o.id = "f".repeat(32); }, o => { o.kind = "vm_start"; },
    o => { o.instanceId = "f".repeat(32); }, o => { o.result.providerScopeId = "f".repeat(64); },
    o => { o.result.planHash = "f".repeat(64); }, o => { o.result.guestReady = false; },
    o => { o.result.guest.cloudInitExitCode = 2; }, o => { o.result.guest.cloudInitClean = false; }]) {
    const operation = ready(); change(operation);
    assert.throws(() => validateCreationOperation(operation, request));
  }
});
