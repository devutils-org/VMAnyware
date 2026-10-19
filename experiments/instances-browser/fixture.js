// IPC simulation only; no native commands, user agents or infrastructure are accessed.
(() => {
  const calls = [], checks = [];
  const instanceId = "a".repeat(32), operationId = "b".repeat(32), scope = "c".repeat(64);
  const instance = { id: instanceId, definitionName: "fixture-definition", definitionHash: "d".repeat(64), definitionRevision: 1,
    providerReference: { provider: "proxmox", scopeId: scope, node: "fixture-node", vmid: 202 }, status: "registered", createdAt: "2026-10-18T12:00:00Z" };
  const operation = (status, failure = null) => ({ id: operationId, kind: "instance_observe", instanceId, status, failure,
    result: status === "succeeded" ? { instanceId, providerScopeId: scope, observation: { node: "fixture-node", vmid: 202, status: "running", qmpStatus: "paused" } } : null,
    createdAt: "2026-10-18T12:00:00Z", updatedAt: "2026-10-18T12:00:01Z" });
  let lists = 0, reads = 0, begins = 0, lateBegin, lateKey, lateListReject;
  let pauseList = false, delayLifecycle = false, lateLifecycle;
  let lifecycleEnabled = false, lifecycleAction = "start", lifecycleBegins = 0, lifecycleReads = 0, lostLifecycleKey, lateLifecycleAction;
  const lifecycleReport = () => ({ instanceId, providerScopeId: scope, definitionHash: instance.definitionHash,
    expectedMarker: `vmanyware-${instanceId}`, configDigest: "e".repeat(40),
    observation: { node: "fixture-node", vmid: 202, status: lifecycleEnabled && lifecycleAction === "start" ? "stopped" : "running", qmpStatus: lifecycleEnabled ? (lifecycleAction === "start" ? null : "running") : "paused" },
    checks: ["instance_marker", "not_template", "unlocked", "no_pending_changes", "configuration_unchanged"].map(code => ({ code, status: !lifecycleEnabled && code === "instance_marker" ? "failed" : "passed", message: "RAW_FIXTURE_MESSAGE" })),
    startPreconditions: lifecycleEnabled && lifecycleAction === "start" ? "passed" : "failed", shutdownPreconditions: lifecycleEnabled && lifecycleAction === "shutdown" ? "passed" : "failed", executionEnabled: lifecycleEnabled });
  const lifecycleOperation = status => ({ id: "f".repeat(32), kind: lifecycleAction === "start" ? "vm_start" : "vm_shutdown", instanceId, status,
    providerTaskKnown: status === "succeeded", failure: status === "uncertain" ? "provider_submission_unconfirmed" : null,
    result: status === "succeeded" ? { action: lifecycleAction, instanceId, providerScopeId: scope, taskStatus: "succeeded",
      observation: { node: "fixture-node", vmid: 202, status: lifecycleAction === "start" ? "running" : "stopped", qmpStatus: null } } : null,
    updatedAt: "2026-10-18T12:00:01Z" });
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = { invoke: async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "ssh_host_support") return false;
    if (command === "list_instances") {
      if (pauseList) return new Promise((_, reject) => { lateListReject = reject; });
      return ++lists === 1 ? [] : [instance];
    }
    if (command === "check_instance_lifecycle") {
      if (delayLifecycle) return new Promise(resolve => { lateLifecycle = resolve; });
      return lifecycleReport();
    }
    if (command === "begin_lifecycle_action") {
      lifecycleBegins++;
      const saved = JSON.parse(localStorage.getItem(`vmanyware.instance.lifecycle.${args.port}.${instanceId}`));
      if (saved.requestKey !== args.requestKey || JSON.stringify(saved.approval) !== JSON.stringify(args.approval)) throw Error("Intent was not persisted before dispatch");
      if (args.approval.action !== lifecycleAction || args.approval.instanceId !== instanceId || args.approval.node !== "fixture-node" || args.approval.vmid !== 202 || args.approval.configDigest !== "e".repeat(40) || args.approval.confirmed !== true) throw Error("Wrong confirmation inputs");
      if (lifecycleBegins === 1) return lifecycleOperation("queued");
      if (lifecycleBegins === 2) { lostLifecycleKey = args.requestKey; throw "SIMULATED_TRANSPORT_LOSS"; }
      if (lifecycleBegins === 3) {
        if (args.requestKey !== lostLifecycleKey) throw Error("Resume replaced a lifecycle key");
        return lifecycleOperation("uncertain");
      }
      if (lifecycleBegins === 4) return new Promise(resolve => { lateLifecycleAction = resolve; });
      if (lifecycleBegins === 5 && lifecycleAction === "shutdown") return lifecycleOperation("queued");
      throw Error("Unexpected new lifecycle submission");
    }
    if (command === "read_lifecycle_operation") {
      lifecycleReads++;
      if (args.id !== "f".repeat(32) || args.approval.instanceId !== instanceId) throw Error("Unrelated lifecycle read");
      return lifecycleOperation("succeeded");
    }
    if (command === "begin_instance_refresh") {
      begins++;
      if (begins === 1) return operation("queued");
      if (begins === 2) return operation("failed", "provider_scope_changed");
      if (begins === 3) { lateKey = args.requestKey; return new Promise(resolve => { lateBegin = resolve; }); }
      if (begins === 4) {
        if (args.requestKey !== lateKey) throw new Error("Resume replaced the pending request key");
        return operation("succeeded");
      }
      throw new Error("Unexpected additional observation request");
    }
    if (command === "read_instance_operation") return operation(++reads === 1 ? "running" : "succeeded");
    if (command === "list_operations") return [{ id: operationId, kind: "instance_observe", instanceId, status: "succeeded", failure: null, createdAt: "2026-10-18", updatedAt: "2026-10-18" }];
    throw new Error(`Unexpected fixture command: ${command}`);
  } };
  const check = (value, message) => { if (!value) throw new Error(message); };
  const wait = async predicate => {
    const until = Date.now() + 6000;
    while (!predicate()) {
      if (Date.now() >= until) throw new Error(`Timed out after [${checks.join(", ")}]; buttons: ${[...document.querySelectorAll("button")].map(node => node.textContent).join(" | ")}; commands: ${calls.map(call => call.command).join(", ")}`);
      await new Promise(resolve => setTimeout(resolve, 10));
    }
  };
  const button = text => [...document.querySelectorAll("button")].find(node => node.textContent === text);
  const click = async label => {
    await wait(() => button(label) && !button(label).disabled);
    button(label).click();
  };
  const text = () => document.body.textContent;
  const port = value => {
    const input = document.getElementById("port");
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  };
  // Click at the first enabled DOM commit, before adding a polling delay. This
  // exposes request invalidation during StrictMode's initial effect cleanup.
  const firstLoad = new Promise((resolve, reject) => {
    const observer = new MutationObserver(() => {
      const node = button("Load registered instances");
      if (!node || node.disabled) return;
      observer.disconnect();
      try {
        check(!calls.some(call => call.command === "list_instances"), "Instance list loaded automatically");
        node.click(); resolve();
      } catch (error) { reject(error); }
    });
    observer.observe(document, { childList: true, subtree: true, attributes: true, attributeFilter: ["disabled"] });
  });
  (async () => {
    await firstLoad;
    await wait(() => text().includes("No instances have been registered."));
    checks.push("explicit empty list");
    await click("Load registered instances");
    await wait(() => button("Observe VM 202"));
    check(!calls.some(call => call.command === "check_instance_lifecycle"), "Lifecycle check ran automatically");
    await click("Check lifecycle for VM 202");
    await wait(() => text().includes("Lifecycle preconditions for VM 202"));
    check(text().includes("Instance tag matches: failed"), "Missing lifecycle blocker");
    check(text().includes("Service lifecycle execution: disabled."), "Disabled capability was not displayed");
    check(!text().includes("RAW_FIXTURE_MESSAGE"), "Raw server check message was displayed");
    check(!button("Start VM") && !button("Shutdown VM"), "Preflight exposed mutation controls");
    checks.push("explicit read-only lifecycle blockers");
    lifecycleEnabled = true;
    await click("Check lifecycle for VM 202");
    await wait(() => button("Review start VM 202"));
    await click("Review start VM 202");
    check(button("Confirm start VM 202").disabled, "Confirmation did not require consent");
    check(lifecycleBegins === 0, "Review dispatched automatically");
    await click("Cancel lifecycle confirmation");
    check(lifecycleBegins === 0, "Cancellation dispatched an action");
    await click("Review start VM 202");
    check(text().includes(`Provider scope: ${scope}`) && text().includes("Configuration: " + "e".repeat(40)), "Confirmation omitted exact scope/revision");
    document.querySelector('.lifecycle-actions input[type="checkbox"]').click();
    await click("Confirm start VM 202");
    await wait(() => text().includes("Lifecycle action: succeeded"));
    check(text().includes("Guest and SSH readiness are not verified"), "Lifecycle result claimed readiness");
    check(!button("Review start VM 202"), "Saved intent allowed a fresh action");
    await click("Acknowledge lifecycle result");
    check(!button("Review start VM 202"), "Acknowledgement reused stale preflight");
    checks.push("explicit target confirmation, cancellation and successful monitored action");
    await click("Check lifecycle for VM 202");
    await click("Review start VM 202");
    document.querySelector('.lifecycle-actions input[type="checkbox"]').click();
    await click("Confirm start VM 202");
    await wait(() => text().includes("Cannot confirm the latest outcome"));
    check(!text().includes("SIMULATED_TRANSPORT_LOSS"), "Raw lifecycle transport error displayed");
    port("9090");
    await wait(() => !button("Observe VM 202"));
    port("8080");
    await click("Load registered instances");
    await click("Resume lifecycle monitoring");
    await wait(() => text().includes("Lifecycle action: uncertain"));
    check(!button("Acknowledge lifecycle result") && !button("Review start VM 202"), "Uncertain result permitted a new request");
    check(lifecycleBegins === 3, "Lost reply was not resumed exactly once with the same key");
    await click("Resume lifecycle monitoring");
    await wait(() => text().includes("Lifecycle action: succeeded"));
    check(lifecycleBegins === 3 && lifecycleReads === 2, "Known lifecycle intent was resubmitted instead of read");
    await click("Acknowledge lifecycle result");
    checks.push("persisted lost-reply same-key resume and uncertain GET-only reconciliation");
    await click("Check lifecycle for VM 202");
    await click("Review start VM 202");
    document.querySelector('.lifecycle-actions input[type="checkbox"]').click();
    await click("Confirm start VM 202");
    await wait(() => lateLifecycleAction);
    port("9090");
    await wait(() => !button("Observe VM 202"));
    lateLifecycleAction(lifecycleOperation("queued"));
    await new Promise(resolve => setTimeout(resolve, 50));
    check(!text().includes("Lifecycle action:"), "Late lifecycle result updated a different port view");
    const retained = JSON.parse(localStorage.getItem(`vmanyware.instance.lifecycle.8080.${instanceId}`));
    check(retained.operationId === "f".repeat(32), "Late lifecycle acknowledgement lost its ID");
    port("8080");
    await click("Load registered instances");
    await click("Resume lifecycle monitoring");
    await wait(() => text().includes("Lifecycle action: succeeded"));
    check(lifecycleBegins === 4 && lifecycleReads === 3, "Late acknowledgement resumed through another submission");
    await click("Acknowledge lifecycle result");
    checks.push("late lifecycle acknowledgement persists scoped identity without stale display");
    lifecycleAction = "shutdown";
    await click("Check lifecycle for VM 202");
    await click("Review shutdown VM 202");
    check(text().includes("Confirm graceful shutdown"), "Shutdown confirmation omitted graceful behavior");
    document.querySelector('.lifecycle-actions input[type="checkbox"]').click();
    await click("Confirm shutdown VM 202");
    await wait(() => text().includes("Lifecycle action: succeeded"));
    check(text().includes("power is stopped"), "Shutdown result did not expose separate power observation");
    await click("Acknowledge lifecycle result");
    lifecycleEnabled = false; lifecycleAction = "start";
    checks.push("explicit graceful shutdown confirmation and monitored stopped power");

    await click("Observe VM 202");
    await wait(() => text().includes("Instance observation: succeeded"));
    check(text().includes("VM 202: running · QEMU: paused"), "Power/QMP observation missing");
    check(text().includes("Guest readiness and ownership remain unverified"), "Observation implied guest readiness");
    checks.push("queued polling and paused VM");
    await click("Observe VM 202");
    await wait(() => text().includes("endpoint differs from the registered instance"));
    check(!text().includes("VM 202: running"), "Previous observation survived failed read");
    checks.push("scope failure clears prior result");
    await click("Observe VM 202");
    await wait(() => lateBegin);
    port("9090");
    await wait(() => button("Load registered instances") && !button("Observe VM 202"));
    lateBegin(operation("succeeded"));
    await new Promise(resolve => setTimeout(resolve, 50));
    check(!text().includes("VM 202: running"), "Late result appeared after port edit");
    port("8080");
    await wait(() => document.getElementById("port").value === "8080");
    await click("Load registered instances");
    await wait(() => button("Observe VM 202"));
    await click("Observe VM 202");
    await wait(() => text().includes("Instance observation: succeeded"));
    checks.push("port edit rejects stale result and resumes same key");
    delayLifecycle = true;
    await click("Check lifecycle for VM 202");
    await wait(() => lateLifecycle);
    port("9090");
    await wait(() => button("Load registered instances") && !button("Observe VM 202"));
    lateLifecycle(lifecycleReport());
    await new Promise(resolve => setTimeout(resolve, 50));
    check(!text().includes("Lifecycle preconditions for VM 202"), "Stale preflight appeared after port edit");
    checks.push("stale lifecycle report suppression");
    port("8080");
    await wait(() => document.getElementById("port").value === "8080");
    await click("Load registered instances");
    await wait(() => button("Observe VM 202"));
    pauseList = true;
    await click("Load registered instances");
    await wait(() => lateListReject);
    port("9090");
    await wait(() => button("Load registered instances") && !button("Observe VM 202"));
    lateListReject("STALE_FIXTURE_ERROR");
    await new Promise(resolve => setTimeout(resolve, 50));
    check(!text().includes("STALE_FIXTURE_ERROR"), "Late list error appeared after port edit");
    await click("Load operation history");
    await wait(() => text().includes("1 recent operations loaded."));
    check(text().includes("Instance observation"), "History omitted observation kind");
    check(text().includes(`Instance: ${instanceId}`), "History omitted instance identity");
    checks.push("stale error suppression and operation history");
    return { ok: true, checks };
  })().catch(error => ({ ok: false, error: `${error?.message ?? error}\n${error?.stack ?? ""}` }))
    .then(report => fetch("/__instance_fixture/result", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(report) }));
})();
