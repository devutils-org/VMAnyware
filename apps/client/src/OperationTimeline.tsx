import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAutoRun } from "./actionFlow";
import { Icon, type IconName } from "./icons";
import { RemoteDeletionRecovery } from "./RemoteDeletionRecovery";
import { RemoteLifecycleRecovery } from "./RemoteLifecycleRecovery";
import { RemoteRecovery } from "./RemoteRecovery";
import { RemoteRecreationRecovery } from "./RemoteRecreationRecovery";
import { RemoteRestoreRecovery } from "./RemoteRestoreRecovery";
import { RemoteSnapshotRecovery } from "./RemoteSnapshotRecovery";

type Environment = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number }; status: "registered" | "deleted" };
type Kind = "create" | "start" | "shutdown" | "snapshot" | "restore" | "delete" | "recreate";
type Entry = { id: string; kind: Kind; label: string; status: string; createdAt: string; environment: Environment | null };
type Item = { id: string; status: string; createdAt: string };
const statuses = ["queued", "running", "succeeded", "failed", "interrupted", "uncertain"];
const icons: Record<Kind, IconName> = { create: "plus", start: "play", shutdown: "power", snapshot: "camera", restore: "repeat", delete: "trash", recreate: "repeat" };
const valid = (item: Item) => /^[0-9a-f]{32}$/.test(item.id) && statuses.includes(item.status) && typeof item.createdAt === "string";

/**
 * One timeline of retained operations, global or for one machine. It only reads
 * history; opening an entry shows its saved detail without repeating any action.
 */
export function OperationTimeline({ connectionId, environments, includeCreations, endpoint, disabled, onSshTarget }: { connectionId: number; environments: Environment[]; includeCreations: boolean; endpoint: string; disabled: boolean; onSshTarget: (host: string, username: string) => void }) {
  const [entries, setEntries] = useState<Entry[] | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState(""), [open, setOpen] = useState<string | null>(null);
  async function load() {
    setBusy(true); setError("");
    const reads: Promise<Entry[]>[] = [];
    const history = <T extends Item>(command: string, args: object, map: (item: T) => Omit<Entry, "id" | "status" | "createdAt">) =>
      invoke<{ operations: T[] }>(command, { connectionId, ...args }).then(result => result.operations.filter(valid).map(item => ({ id: item.id, status: item.status, createdAt: item.createdAt, ...map(item) })));
    for (const environment of environments) {
      const instanceId = environment.id;
      reads.push(history<Item & { action: "start" | "shutdown" }>("read_remote_lifecycle_history", { instanceId }, item => ({ kind: item.action, label: item.action === "start" ? "Start" : "Shut down", environment })));
      reads.push(history<Item & { name: string }>("read_remote_snapshot_history", { instanceId }, item => ({ kind: "snapshot", label: `Snapshot ${item.name}`, environment })));
      reads.push(history<Item & { snapshotName: string }>("read_remote_restore_history", { instanceId }, item => ({ kind: "restore", label: `Restore ${item.snapshotName}`, environment })));
      reads.push(history<Item>("read_remote_deletion_history", { instanceId }, () => ({ kind: "delete", label: "Delete", environment })));
      if (environment.status === "deleted") reads.push(history<Item>("read_remote_recreation_history", { sourceInstanceId: instanceId }, () => ({ kind: "recreate", label: "Recreate", environment })));
    }
    if (includeCreations) reads.push(invoke<(Item & { instanceId: string | null })[]>("read_remote_creations", { connectionId }).then(items => items.filter(valid).map(item => ({
      id: item.id, status: item.status, createdAt: item.createdAt, kind: "create" as const, label: "Create",
      environment: environments.find(environment => environment.id === item.instanceId) ?? null,
    }))));
    const results = await Promise.allSettled(reads);
    const merged = new Map<string, Entry>();
    for (const result of results) if (result.status === "fulfilled") for (const entry of result.value) merged.set(entry.id, entry);
    setEntries([...merged.values()].sort((a, b) => b.createdAt.localeCompare(a.createdAt)).slice(0, 200));
    // Some histories do not apply to every machine state; only a total failure is an error.
    if (results.length > 0 && results.every(result => result.status === "rejected")) setError("Operation history could not be read. Refresh to retry.");
    setBusy(false);
  }
  useAutoRun(true, () => void load(), [connectionId, environments.map(environment => environment.id + environment.status).join()]);
  return <div className="timeline">
    <div className="timeline-head"><span className="muted">{entries ? `${entries.length} operation${entries.length === 1 ? "" : "s"}` : busy ? "Loading…" : ""}</span>
      <button type="button" className="ghost" disabled={disabled || busy} onClick={() => void load()}><Icon name="refresh" />Refresh</button></div>
    {error && <p role="alert" className="error">{error}</p>}
    {entries && entries.length === 0 && <p className="muted">No operations recorded yet.</p>}
    {entries && entries.length > 0 && <ol className="timeline-list">{entries.map(entry => <li key={entry.id} className={open === entry.id ? "open" : ""}>
      <button type="button" className="timeline-row" aria-expanded={open === entry.id} onClick={() => setOpen(open === entry.id ? null : entry.id)}>
        <span className={`timeline-icon ${entry.kind}`}><Icon name={icons[entry.kind]} /></span>
        <span className="timeline-label">{entry.label}{entry.environment && <span className="muted"> · {entry.environment.definitionName} · VM {entry.environment.providerReference.vmid}</span>}</span>
        <span className={`chip state ${entry.status}`}>{entry.status}</span>
        <time className="muted mono" dateTime={entry.createdAt}>{new Date(entry.createdAt).toLocaleString()}</time>
      </button>
      {open === entry.id && <div className="timeline-detail">{detail(entry)}</div>}
    </li>)}</ol>}
  </div>;
  function detail(entry: Entry) {
    const environment = entry.environment;
    if (entry.kind === "create") return <RemoteRecovery connectionId={connectionId} endpoint={endpoint} disabled={disabled} onSshTarget={onSshTarget} focus={entry.id} />;
    if (!environment) return <p className="muted">The environment of this operation is not listed.</p>;
    if (entry.kind === "start" || entry.kind === "shutdown") return <RemoteLifecycleRecovery connectionId={connectionId} environment={environment} disabled={disabled} focus={entry.id} />;
    if (entry.kind === "snapshot") return <RemoteSnapshotRecovery connectionId={connectionId} environment={environment} disabled={disabled} focus={entry.id} />;
    if (entry.kind === "restore") return <RemoteRestoreRecovery connectionId={connectionId} environment={environment} disabled={disabled} focus={entry.id} />;
    if (entry.kind === "delete") return <RemoteDeletionRecovery connectionId={connectionId} environment={environment} disabled={disabled} focus={entry.id} />;
    return <RemoteRecreationRecovery connectionId={connectionId} source={environment} disabled={disabled} focus={entry.id} />;
  }
}
