import { validateLifecyclePreflight, type InstanceSummary, type LifecyclePreflight } from "./instanceObservation.ts";

export type LifecycleAction = "start" | "shutdown";
export type LifecycleApproval = {
  action: LifecycleAction; instanceId: string; providerScopeId: string; node: string; vmid: number;
  definitionHash: string; configDigest: string; confirmed: true;
};
export type PendingLifecycle = {
  version: 1; port: number; requestKey: string; approval: LifecycleApproval; operationId?: string;
};
export type LifecycleOperation = {
  id: string; kind: "vm_start" | "vm_shutdown"; instanceId: string;
  status: "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain";
  providerTaskKnown: boolean; failure: string | null; updatedAt: string;
  result: null | { action: LifecycleAction; instanceId: string; providerScopeId: string; taskStatus: "succeeded";
    observation: { node: string; vmid: number; status: "running" | "stopped"; qmpStatus: string | null } };
};
const hex = (value: unknown, size: number): value is string => typeof value === "string" && value.length === size && /^[a-f0-9]+$/.test(value);
const invalid = "The saved lifecycle request or returned operation is invalid. Review operation history before another action.";

export function validatePendingLifecycle(pending: PendingLifecycle, instance?: InstanceSummary) {
  const a = pending?.approval;
  if (!a || pending.version !== 1 || !Number.isInteger(pending.port) || pending.port < 1 || pending.port > 65535
    || typeof pending.requestKey !== "string" || !/^[a-zA-Z0-9_.-]{1,64}$/.test(pending.requestKey)
    || !["start", "shutdown"].includes(a.action) || a.confirmed !== true
    || !hex(a.instanceId, 32) || !hex(a.providerScopeId, 64) || !hex(a.definitionHash, 64) || !hex(a.configDigest, 40)
    || typeof a.node !== "string" || !/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,62}$/.test(a.node)
    || !Number.isInteger(a.vmid) || a.vmid < 100 || a.vmid > 999999999
    || (pending.operationId !== undefined && !hex(pending.operationId, 32))
    || (instance && (a.instanceId !== instance.id || a.definitionHash !== instance.definitionHash
      || a.providerScopeId !== instance.providerReference.scopeId || a.node !== instance.providerReference.node || a.vmid !== instance.providerReference.vmid))) throw invalid;
}

/** Call only after explicit user confirmation; a preflight alone must never call this. */
export function confirmedLifecycle(port: number, instance: InstanceSummary, report: LifecyclePreflight, action: LifecycleAction): PendingLifecycle {
  validateLifecyclePreflight(report, instance);
  const power = report.observation;
  const permitted = action === "start"
    ? report.startPreconditions === "passed" && power.status === "stopped" && (power.qmpStatus === null || power.qmpStatus === "stopped")
    : report.shutdownPreconditions === "passed" && power.status === "running" && power.qmpStatus === "running";
  if (!report.executionEnabled || !permitted || report.checks.some(check => check.status !== "passed")) {
    throw "This action is unavailable. Run a fresh lifecycle check and review its blockers.";
  }
  const pending: PendingLifecycle = { version: 1, port, requestKey: crypto.randomUUID(), approval: {
    action, instanceId: instance.id, providerScopeId: instance.providerReference.scopeId,
    node: instance.providerReference.node, vmid: instance.providerReference.vmid,
    definitionHash: instance.definitionHash, configDigest: report.configDigest, confirmed: true,
  } };
  validatePendingLifecycle(pending, instance);
  return pending;
}

const failures = ["provider_not_configured", "provider_unauthorized", "provider_forbidden", "provider_timeout", "provider_unavailable",
  "process_interrupted", "provider_scope_changed", "instance_reference_invalid", "lifecycle_disabled", "approval_expired",
  "lifecycle_target_changed", "lifecycle_preconditions_failed", "provider_submission_unconfirmed", "provider_task_failed",
  "provider_task_unconfirmed", "power_state_unconfirmed"];
export function validateLifecycleOperation(operation: LifecycleOperation, pending: PendingLifecycle) {
  const a = pending.approval, r = operation.result;
  const success = operation.status === "succeeded";
  const failed = ["failed", "interrupted", "uncertain"].includes(operation.status);
  if (!hex(operation.id, 32) || (pending.operationId !== undefined && operation.id !== pending.operationId)
    || operation.kind !== (a.action === "start" ? "vm_start" : "vm_shutdown") || operation.instanceId !== a.instanceId
    || !["queued", "running", "succeeded", "failed", "interrupted", "uncertain"].includes(operation.status)
    || typeof operation.providerTaskKnown !== "boolean" || success !== (r !== null)
    || failed !== (operation.failure !== null) || (failed && !failures.includes(operation.failure!))
    || (success && (!operation.providerTaskKnown || !r || r.action !== a.action || r.instanceId !== a.instanceId
      || r.providerScopeId !== a.providerScopeId || r.taskStatus !== "succeeded" || r.observation.node !== a.node
      || r.observation.vmid !== a.vmid || r.observation.status !== (a.action === "start" ? "running" : "stopped")))) throw invalid;
}

type Invoke = <T>(command: string, args: Record<string, unknown>) => Promise<T>;
type Options = { persist: (pending: PendingLifecycle) => void; now?: () => number; wait?: () => Promise<void> };
/** One original request, bounded polling. Never replaces its key or approval after any error. */
export async function monitorLifecycle(pending: PendingLifecycle, invoke: Invoke,
  onOperation: (operation: LifecycleOperation) => void, active: () => boolean, options: Options,
): Promise<"complete" | "uncertain" | "paused" | "stale"> {
  const request = { ...pending, approval: { ...pending.approval } };
  validatePendingLifecycle(request);
  if (!active()) return "stale";
  const now = options.now ?? (() => performance.now());
  const wait = options.wait ?? (() => new Promise(resolve => setTimeout(resolve, 500)));
  const deadline = now() + 60000;
  // Persistence must succeed before transport. A caller must not silently drop a pending write.
  options.persist({ ...request, approval: { ...request.approval } });
  let operation = await invoke<LifecycleOperation>(request.operationId ? "read_lifecycle_operation" : "begin_lifecycle_action",
    request.operationId ? { port: request.port, id: request.operationId, approval: request.approval }
      : { port: request.port, requestKey: request.requestKey, approval: request.approval });
  for (;;) {
    validateLifecycleOperation(operation, request);
    request.operationId = operation.id;
    // Retain an acknowledged ID even if the user left during the HTTP request.
    options.persist({ ...request, approval: { ...request.approval } });
    if (!active()) return "stale";
    onOperation(operation);
    if (operation.status === "uncertain") return "uncertain";
    if (operation.status !== "queued" && operation.status !== "running") return "complete";
    if (now() >= deadline) return "paused";
    await wait();
    if (!active()) return "stale";
    operation = await invoke<LifecycleOperation>("read_lifecycle_operation", { port: request.port, id: request.operationId, approval: request.approval });
  }
}
