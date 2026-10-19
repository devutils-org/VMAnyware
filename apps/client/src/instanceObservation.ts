export type InstanceSummary = {
  id: string;
  definitionName: string;
  definitionHash: string;
  definitionRevision: number;
  providerReference: { provider: string; scopeId: string; node: string; vmid: number };
  status: "registered";
  createdAt: string;
};
export type ObservationOperation = {
  id: string;
  kind: "instance_observe";
  instanceId: string;
  status: "queued" | "running" | "succeeded" | "failed" | "interrupted";
  result: null | { instanceId: string; providerScopeId: string; observation: {
    node: string; vmid: number; status: "running" | "stopped"; qmpStatus: string | null;
  } };
  failure: string | null;
  updatedAt: string;
};
type Invoke = <T>(command: string, args: Record<string, unknown>) => Promise<T>;
type Options = {
  now?: () => number;
  wait?: () => Promise<void>;
  loadKey?: (key: string) => string | null;
  saveKey?: (key: string, value: string | null) => void;
};
function loadKey(key: string): string | null {
  try { return localStorage.getItem(key); } catch { return null; }
}
function saveKey(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch { /* Durable server records remain available without local resume caching. */ }
}
function validate(operation: ObservationOperation, instance: InstanceSummary, expectedId?: string) {
  const result = operation.result;
  if (operation.kind !== "instance_observe" || operation.instanceId !== instance.id
    || !/^[a-f0-9]{32}$/.test(operation.id) || (expectedId !== undefined && operation.id !== expectedId)
    || !["queued", "running", "succeeded", "failed", "interrupted"].includes(operation.status)
    || (operation.status === "succeeded" && !result)
    || (result !== null && (operation.status !== "succeeded" || result.instanceId !== instance.id
      || result.providerScopeId !== instance.providerReference.scopeId
      || result.observation.node !== instance.providerReference.node
      || result.observation.vmid !== instance.providerReference.vmid
      || !["running", "stopped"].includes(result.observation.status)))) {
    throw "The control plane returned an unrelated or invalid instance observation.";
  }
}

/** Explicit read monitoring. Leaving the view stops polling, not the durable job. */
export async function observeInstance(
  port: number, instance: InstanceSummary, invoke: Invoke,
  onOperation: (operation: ObservationOperation) => void, active: () => boolean,
  options: Options = {},
): Promise<"complete" | "paused" | "stale"> {
  if (!active()) return "stale";
  const now = options.now ?? Date.now;
  const wait = options.wait ?? (() => new Promise(resolve => setTimeout(resolve, 500)));
  const read = options.loadKey ?? loadKey;
  const save = options.saveKey ?? saveKey;
  const cache = `vmanyware.instance.observe.${port}.${instance.id}`;
  const existing = read(cache);
  const requestKey = existing && /^[a-zA-Z0-9_.-]{1,64}$/.test(existing) ? existing : crypto.randomUUID();
  save(cache, requestKey);
  const deadline = now() + 60_000;
  let operation = await invoke<ObservationOperation>("begin_instance_refresh", { port, instanceId: instance.id, requestKey });
  const operationId = operation.id;
  for (;;) {
    if (!active()) return "stale";
    validate(operation, instance, operationId);
    onOperation(operation);
    if (operation.status !== "queued" && operation.status !== "running") {
      save(cache, null);
      return "complete";
    }
    if (now() >= deadline) return "paused";
    await wait();
    if (!active()) return "stale";
    operation = await invoke<ObservationOperation>("read_instance_operation", { port, id: operationId, instanceId: instance.id });
  }
}

export type LifecycleStatus = "passed" | "failed" | "unknown";
export type LifecycleCheckCode = "instance_marker" | "not_template" | "unlocked" | "no_pending_changes" | "configuration_unchanged";
export type LifecyclePreflight = {
  instanceId: string;
  providerScopeId: string;
  definitionHash: string;
  expectedMarker: string;
  configDigest: string;
  observation: { node: string; vmid: number; status: "running" | "stopped"; qmpStatus: string | null };
  checks: { code: LifecycleCheckCode; status: LifecycleStatus; message: string }[];
  startPreconditions: LifecycleStatus;
  shutdownPreconditions: LifecycleStatus;
  executionEnabled: boolean;
};

/** Bind the read-only report to the selected saved association, never to current form input. */
export function validateLifecyclePreflight(report: LifecyclePreflight, instance: InstanceSummary) {
  const codes = ["instance_marker", "not_template", "unlocked", "no_pending_changes", "configuration_unchanged"];
  const statuses = ["passed", "failed", "unknown"];
  if (report.instanceId !== instance.id || report.providerScopeId !== instance.providerReference.scopeId
    || report.definitionHash !== instance.definitionHash || report.expectedMarker !== `vmanyware-${instance.id}`
    || !/^[a-fA-F0-9]{40}$/.test(report.configDigest) || typeof report.executionEnabled !== "boolean"
    || report.observation.node !== instance.providerReference.node || report.observation.vmid !== instance.providerReference.vmid
    || !["running", "stopped"].includes(report.observation.status)
    || !statuses.includes(report.startPreconditions) || !statuses.includes(report.shutdownPreconditions)
    || !Array.isArray(report.checks) || report.checks.length !== codes.length
    || new Set(report.checks.map(check => check.code)).size !== codes.length
    || report.checks.some(check => !codes.includes(check.code) || !statuses.includes(check.status))) {
    throw "The control plane returned an unrelated or invalid lifecycle preflight.";
  }
}
