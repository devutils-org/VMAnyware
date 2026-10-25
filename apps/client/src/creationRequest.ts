import type { Plan, Preflight } from "./ProvisioningPreview";

export type CreationOperation = {
  id: string; kind: "vm_provision"; instanceId?: string | null;
  status: "queued" | "running" | "uncertain" | "interrupted" | "failed" | "succeeded";
  failure: string | null;
  result: null | { instanceId: string; providerScopeId: string; planHash: string;
    guestReady: boolean; guest: { ipv4: string; cloudInitExitCode: number; cloudInitClean: boolean } };
};
export type PendingCreation = {
  version: 1; port: number; requestKey: string; plan: Plan; operationId?: string; instanceId?: string;
};
const invalid = "Saved creation state or response is invalid. Review operation history before another request.";
const hex = (s: unknown, n: number): s is string => typeof s === "string" && s.length === n && /^[a-f0-9]+$/.test(s);
const identifier = (s: unknown, n: number): s is string => typeof s === "string" && s.length <= n && /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(s);
const integer = (n: number, min: number, max: number) => Number.isInteger(n) && n >= min && n <= max;
export function validatePendingCreation(p: PendingCreation) {
  const plan = p?.plan;
  if (!p || p.version !== 1 || !integer(p.port, 1, 65535)
    || !identifier(p.requestKey, 64) || !plan || !hex(plan.definitionHash, 64) || !hex(plan.planHash, 64)
    || !hex(plan.providerScopeId, 64) || !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(plan.definitionName) || plan.definitionName.length > 63
    || plan.creationEnabled !== true || !plan.source || !plan.target
    || !integer(plan.source.templateVmid, 100, 999999999) || !integer(plan.target.vmid, 100, 999999999)
    || plan.source.templateVmid === plan.target.vmid || !identifier(plan.target.node, 63)
    || !identifier(plan.target.storage, 64) || !identifier(plan.target.bridge, 15)
    || !identifier(plan.target.guestUser, 32) || typeof plan.source.templateVersion !== "string"
    || !plan.source.templateVersion.length || plan.source.templateVersion.length > 128 || !plan.target.resources
    || !integer(plan.target.resources.cpu, 1, 64) || !integer(plan.target.resources.memoryMiB, 256, 262144)
    || !integer(plan.target.resources.diskGiB, 4, 4096) || !integer(plan.sshKeyCount, 1, 16)
    || !Array.isArray(plan.packages) || plan.packages.length > 128 || !plan.packages.every(p => typeof p === "string")
    || typeof plan.snapshotAfterProvisioning !== "boolean"
    || (p.operationId !== undefined && !hex(p.operationId, 32))
    || (p.instanceId !== undefined && !hex(p.instanceId, 32))) throw invalid;
}
const requiredChecks = ["template", "source_disk", "cloud_init_drive", "storage", "storage_capacity", "bridge", "target_vmid"];
export function creationAvailable(plan: Plan | null, preflight: Preflight | null): boolean {
  return !!plan?.creationEnabled && !!preflight?.creationEnabled && preflight.planHash === plan.planHash
    && requiredChecks.every(code => preflight.checks.filter(c => c.code === code && c.status === "passed").length === 1)
    && preflight.checks.every(c => c.status !== "failed");
}
/** Construct only after the user reviews the exact plan and explicitly confirms it. */
export function confirmedCreation(port: number, plan: Plan, preflight: Preflight, confirmed: boolean): PendingCreation {
  if (!confirmed || !creationAvailable(plan, preflight)) {
    throw "Run a fresh successful preflight and confirm this exact creation plan.";
  }
  const pending: PendingCreation = { version: 1, port, requestKey: crypto.randomUUID(), plan: structuredClone(plan) };
  validatePendingCreation(pending);
  return pending;
}
export function validateCreationOperation(operation: CreationOperation, pending: PendingCreation) {
  if (!operation || !hex(operation.id, 32) || operation.kind !== "vm_provision"
    || (pending.operationId !== undefined && operation.id !== pending.operationId)
    || (operation.instanceId != null && !hex(operation.instanceId, 32))
    || (pending.instanceId !== undefined && operation.instanceId !== pending.instanceId)
    || !["queued", "running", "uncertain", "interrupted", "failed", "succeeded"].includes(operation.status)) throw invalid;
  const succeeded = operation.status === "succeeded";
  const failed = ["uncertain", "interrupted", "failed"].includes(operation.status);
  if (failed !== (operation.failure !== null) || (failed && typeof operation.failure !== "string")
    || succeeded !== (operation.result !== null)) throw invalid;
  if (succeeded) {
    const r = operation.result!;
    if (!hex(r.instanceId, 32) || (operation.instanceId != null && r.instanceId !== operation.instanceId)
      || r.providerScopeId !== pending.plan.providerScopeId || r.planHash !== pending.plan.planHash
      || r.guestReady !== true || !r.guest || r.guest.cloudInitClean !== true || r.guest.cloudInitExitCode !== 0
      || typeof r.guest.ipv4 !== "string" || !/^\d{1,3}(\.\d{1,3}){3}$/.test(r.guest.ipv4)
      || r.guest.ipv4.split(".").some(n => Number(n) > 255)) throw invalid;
  }
}
type Invoke = <T>(command: string, args: Record<string, unknown>) => Promise<T>;
type Options = { persist: (p: PendingCreation) => void; now?: () => number; wait?: () => Promise<void> };
export async function monitorCreation(pending: PendingCreation, invoke: Invoke,
  onOperation: (o: CreationOperation) => void, active: () => boolean, options: Options,
): Promise<"complete" | "uncertain" | "paused" | "stale"> {
  const request = structuredClone(pending);
  validatePendingCreation(request);
  if (!active()) return "stale";
  const now = options.now ?? (() => performance.now());
  const deadline = now() + 60000;
  const wait = options.wait ?? (() => new Promise(resolve => setTimeout(resolve, 1000)));
  options.persist(structuredClone(request));
  let operation = await invoke<CreationOperation>(request.operationId ? "read_local_creation" : "begin_local_creation",
    request.operationId ? { port: request.port, id: request.operationId, plan: request.plan }
      : { port: request.port, plan: request.plan, requestKey: request.requestKey, confirmed: true });
  for (;;) {
    validateCreationOperation(operation, request);
    request.operationId = operation.id;
    if (operation.instanceId) request.instanceId = operation.instanceId;
    options.persist(structuredClone(request));
    if (!active()) return "stale";
    onOperation(operation);
    if (operation.status === "uncertain") return "uncertain";
    if (operation.status !== "queued" && operation.status !== "running") return "complete";
    if (now() >= deadline) return "paused";
    await wait();
    if (!active()) return "stale";
    operation = await invoke<CreationOperation>("read_local_creation", { port: request.port, id: request.operationId, plan: request.plan });
  }
}
