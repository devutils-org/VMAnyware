import { useEffect, useRef, useState } from "react";
import { RemoteRestoreRecovery } from "./RemoteRestoreRecovery";
import { RemoteSnapshotRecovery } from "./RemoteSnapshotRecovery";
import { invoke } from "@tauri-apps/api/core";
import { useAutoRun } from "./actionFlow";

type Target = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type Snapshot = { name: string; description: string; createdAt: number | null; includesRam: boolean | null; parent: string | null; providerState: string | null };
type Listing = { instanceId: string; providerScopeId: string; node: string; vmid: number; snapshots: Snapshot[] };
export function RemoteSnapshots({ connectionId, environment, disabled, restoreDisabled, onRestore, autoLoad = false, showRecovery = true }: { connectionId: number; environment: Target; disabled: boolean; restoreDisabled?: boolean; onRestore?: (snapshotName: string) => void; autoLoad?: boolean; showRecovery?: boolean }) {
  const [listing, setListing] = useState<Listing | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => () => { generation.current++; }, [connectionId, environment.id]);
  // Opening the snapshot view is the explicit read request.
  useAutoRun(autoLoad, () => void load(), [connectionId, environment.id]);
  async function load() {
    const current = ++generation.current;
    setListing(null); setError(""); setBusy(true);
    try {
      const result = await invoke<Listing>("read_remote_snapshots", { connectionId, instanceId: environment.id });
      if (generation.current !== current) return;
      const target = environment.providerReference;
      if (result.instanceId !== environment.id || result.providerScopeId !== target.scopeId || result.node !== target.node || result.vmid !== target.vmid) throw "The snapshot list belongs to a different target. Reload the remote workspace.";
      setListing(result);
    } catch (failure) { if (generation.current === current) setError(typeof failure === "string" ? failure : "Cannot load snapshots. Please retry."); }
    finally { if (generation.current === current) setBusy(false); }
  }
  return <div className="remote-snapshots" aria-busy={busy}>
    <button type="button" disabled={disabled || busy} onClick={load}>{listing ? "Refresh snapshots" : `List snapshots for VM ${environment.providerReference.vmid}`}</button>
    {busy && <p role="status">Reading snapshots for {environment.definitionName}…</p>}
    {error && <p role="alert" className="error">{error}</p>}
    {listing && <>
      <p>Snapshots: {environment.definitionName} · {listing.node} · VM {listing.vmid}</p>
      <p>Snapshots depend on the original VM and storage. They are not independent backups. Listing does not verify consistency or restore eligibility.</p>
      {listing.snapshots.length === 0 ? <p role="status">No snapshots reported for this VM. Storage support has not been checked.</p> : <ul>{listing.snapshots.map(snapshot => <li key={snapshot.name}>
        <strong>{snapshot.name}</strong>
        {onRestore && <button type="button" aria-label={`Review restore ${snapshot.name} for VM ${listing.vmid}`} disabled={disabled || restoreDisabled} onClick={() => onRestore(snapshot.name)}>Restore…</button>}
        <p>{snapshot.description || "No description."}</p>
        <p>Created: {snapshot.createdAt ? new Date(snapshot.createdAt * 1000).toLocaleString() : "Unknown"} · RAM: {snapshot.includesRam === null ? "Unknown" : snapshot.includesRam ? "Included" : "Not included"}</p>
        {snapshot.parent && <p>Parent: {snapshot.parent}</p>}
        {snapshot.providerState && <p role="alert">Provider state: {snapshot.providerState}. This snapshot may be incomplete.</p>}
      </li>)}</ul>}
    </>}
    {showRecovery && <>
      <RemoteSnapshotRecovery connectionId={connectionId} environment={environment} disabled={disabled} />
      <RemoteRestoreRecovery connectionId={connectionId} environment={environment} disabled={disabled} />
    </>}
  </div>;
}
