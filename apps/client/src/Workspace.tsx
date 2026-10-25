import { useCallback, useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon, type IconName } from "./icons";
import { AppIdentity } from "./AppIdentity";
import { LocalWorkspace } from "./LocalWorkspace";
import { RemoteCreate } from "./RemoteCreate";
import { RemoteDeletion } from "./RemoteDeletion";
import { RemoteDeletionRecovery } from "./RemoteDeletionRecovery";
import { RemoteInstanceDetail } from "./RemoteInstanceDetail";
import { RemoteInstanceStatus } from "./RemoteInstanceStatus";
import { RemoteLifecycle } from "./RemoteLifecycle";
import { RemoteLifecycleRecovery } from "./RemoteLifecycleRecovery";
import { RemoteRecovery } from "./RemoteRecovery";
import { RemoteRecreation } from "./RemoteRecreation";
import { RemoteRecreationRecovery } from "./RemoteRecreationRecovery";
import { RemoteRestoreRecovery } from "./RemoteRestoreRecovery";
import { RemoteSnapshotCreate } from "./RemoteSnapshotCreate";
import { RemoteSnapshotRecovery } from "./RemoteSnapshotRecovery";
import { RemoteSnapshotRestore } from "./RemoteSnapshotRestore";
import { RemoteSnapshots } from "./RemoteSnapshots";
import { SshHostTrust } from "./SshHostTrust";

type Image = { image: string; node: string; templateVmid: number; templateVersion: string; sourceDiskGiB: number; guestUser: string };
type Environment = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number }; status: "registered" | "deleted" };
type Connection = { connectionId: number; endpoint: string };
type View = "environments" | "environment" | "create" | "terminal" | "activity" | "settings";
type Tab = "overview" | "snapshots" | "history";
type Drawer =
  | { kind: "lifecycle"; environment: Environment; action: "start" | "shutdown" }
  | { kind: "snapshot"; environment: Environment }
  | { kind: "restore"; environment: Environment; snapshotName: string }
  | { kind: "deletion"; environment: Environment }
  | { kind: "recreation"; environment: Environment };
type SshTarget = { host: string; username: string; generation: number; instance?: Environment };

// Only the public endpoint and public CA are remembered; the access token never is.
const SAVED_SERVER = "vmanyware.remote.server";
function readSavedServer(): { endpoint: string; caPem: string } | null {
  try {
    const value = JSON.parse(localStorage.getItem(SAVED_SERVER) ?? "null");
    return value && typeof value.endpoint === "string" && typeof value.caPem === "string" ? value : null;
  } catch { return null; }
}
function writeSavedServer(value: { endpoint: string; caPem: string } | null) {
  try { if (value) localStorage.setItem(SAVED_SERVER, JSON.stringify(value)); else localStorage.removeItem(SAVED_SERVER); } catch { /* optional convenience */ }
}
const host = (endpoint: string) => { try { return new URL(endpoint).host; } catch { return endpoint; } };

export function Workspace({ native }: { native: boolean }) {
  const saved = useRef(readSavedServer()).current;
  const [mode, setMode] = useState<"remote" | "local">("remote");
  const [endpoint, setEndpoint] = useState(saved?.endpoint ?? "https://");
  const [token, setToken] = useState("");
  const [caPem, setCaPem] = useState(saved?.caPem ?? "");
  const [remember, setRemember] = useState(true);
  const [hasSaved, setHasSaved] = useState(saved !== null);
  const [connection, setConnection] = useState<Connection | null>(null);
  const [busy, setBusy] = useState(false), [loading, setLoading] = useState(false);
  const [images, setImages] = useState<Image[] | null>(null);
  const [environments, setEnvironments] = useState<Environment[] | null>(null);
  const [error, setError] = useState(""), [loadError, setLoadError] = useState("");
  const [view, setView] = useState<View>("environments");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("overview");
  const [drawer, setDrawer] = useState<Drawer | null>(null), [drawerHidden, setDrawerHidden] = useState(false);
  // Bumped when an action settles so visible reads (power, snapshots) run again.
  const [revision, setRevision] = useState(0);
  const [power, setPower] = useState<{ id: string; status: "running" | "stopped" } | null>(null);
  const [mutationLocked, setMutationLocked] = useState(false);
  const [sshTarget, setSshTarget] = useState<SshTarget | null>(null);
  const [runtime, setRuntime] = useState("");
  const generation = useRef(0), sshGeneration = useRef(0);
  const connectionId = connection?.connectionId;

  useEffect(() => () => { generation.current++; if (native) void invoke("disconnect_remote").catch(() => {}); }, [native]);

  async function connect(event: FormEvent) {
    event.preventDefault();
    const current = ++generation.current;
    const supplied = token;
    setToken(""); setBusy(true); setError(""); resetWorkspace();
    try {
      const result = await invoke<Connection>("connect_remote", { endpoint, token: supplied, caPem: caPem.trim() || null });
      if (generation.current !== current) return;
      if (remember) { writeSavedServer({ endpoint, caPem: caPem.trim() }); setHasSaved(true); }
      setConnection(result); setView("environments");
    } catch (failure) {
      if (generation.current === current) setError(typeof failure === "string" ? failure : "Cannot connect to this server.");
    } finally { if (generation.current === current) setBusy(false); }
  }
  function resetWorkspace() {
    setConnection(null); setImages(null); setEnvironments(null); setLoadError(""); setDrawer(null);
    setMutationLocked(false); setSshTarget(null); setSelectedId(null); setDrawerHidden(false);
  }
  async function disconnect() {
    generation.current++; setBusy(true); setError("");
    try { await invoke("disconnect_remote"); resetWorkspace(); }
    catch { setError("Cannot clear the connection. Close the app to forget it."); }
    finally { setBusy(false); }
  }
  function forgetServer() { writeSavedServer(null); setHasSaved(false); }

  const loadWorkspace = useCallback(async () => {
    if (connectionId === undefined) return;
    const current = ++generation.current;
    setLoading(true); setLoadError("");
    const [imageResult, environmentResult] = await Promise.allSettled([
      invoke<Image[]>("read_remote_images", { connectionId }),
      invoke<Environment[]>("read_remote_environments", { connectionId }),
    ]);
    if (generation.current !== current) return;
    const problems: string[] = [];
    if (imageResult.status === "fulfilled") setImages(imageResult.value);
    else problems.push(typeof imageResult.reason === "string" ? imageResult.reason : "Cannot load templates.");
    if (environmentResult.status === "fulfilled") setEnvironments(environmentResult.value);
    else problems.push(typeof environmentResult.reason === "string" ? environmentResult.reason : "Cannot load environments.");
    setLoadError(problems.join(" "));
    setLoading(false);
  }, [connectionId]);
  useEffect(() => { void loadWorkspace(); }, [loadWorkspace]);
  // Stable: create/recreation effects depend on it. Reads only; never resubmits work.
  const reloadEnvironments = useCallback(() => {
    if (connectionId === undefined) return;
    const current = generation.current;
    invoke<Environment[]>("read_remote_environments", { connectionId })
      .then(value => { if (generation.current === current) setEnvironments(value); })
      .catch(() => {});
  }, [connectionId]);

  // A hidden drawer keeps running work mounted; it disappears once the action settles.
  useEffect(() => { if (drawerHidden && !mutationLocked) { setDrawer(null); setDrawerHidden(false); setRevision(value => value + 1); } }, [drawerHidden, mutationLocked]);
  const closeDrawer = useCallback(() => { if (mutationLocked) setDrawerHidden(true); else setDrawer(null); }, [mutationLocked]);
  const finishDrawer = useCallback(() => { setDrawer(null); setDrawerHidden(false); setRevision(value => value + 1); }, []);
  useEffect(() => {
    if (!drawer || drawerHidden) return;
    const onKey = (event: KeyboardEvent) => { if (event.key === "Escape") closeDrawer(); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [drawer, drawerHidden, closeDrawer]);
  const selected = environments?.find(environment => environment.id === selectedId) ?? null;
  const blocked = busy || loading || mutationLocked;
  function openEnvironment(environment: Environment) { setSelectedId(environment.id); setTab("overview"); setView("environment"); }
  function openDrawer(next: Drawer) { if (!blocked) { setDrawer(next); setDrawerHidden(false); } }
  function selectSshTarget(target: string, username: string) {
    setSshTarget({ host: target, username, generation: ++sshGeneration.current }); setView("terminal");
  }
  function sshForInstance(environment: Environment) {
    if (blocked || environment.status !== "registered") return;
    setSshTarget({ host: "", username: "", generation: ++sshGeneration.current, instance: environment }); setView("terminal");
  }
  // Settings shows the platform the native bridge reports; read when the view opens.
  useEffect(() => {
    if (view !== "settings" || !native || runtime) return;
    invoke<string>("runtime_platform").then(setRuntime).catch(() => setRuntime("unavailable"));
  }, [view, native, runtime]);

  if (mode === "local") return <Shell nav={null} title="Local development service" subtitle="Loopback control plane on this computer" actions={<button type="button" onClick={() => setMode("remote")}><Icon name="back" />Back to remote server</button>}>
    <LocalWorkspace native={native} />
  </Shell>;

  if (!connection) return <div className="connect-screen">
    <form className="connect-card" onSubmit={connect} aria-busy={busy}>
      <div className="brand"><span className="brand-mark" aria-hidden="true">›_</span>VMAnyware</div>
      <h1>Connect to your control plane</h1>
      <p className="muted">Proxmox credentials stay on the server. This device only holds a session token in memory.</p>
      <label htmlFor="remote-endpoint">Server address</label>
      <input id="remote-endpoint" type="url" inputMode="url" autoCapitalize="none" autoCorrect="off" required disabled={busy} value={endpoint} onChange={event => setEndpoint(event.target.value)} placeholder="https://vm.example.com" />
      <label htmlFor="remote-token">Access token</label>
      <input id="remote-token" type="password" autoComplete="off" autoCapitalize="none" autoCorrect="off" spellCheck={false} minLength={32} maxLength={256} required disabled={busy} value={token} onChange={event => setToken(event.target.value.trim())} />
      <details open={saved?.caPem ? true : undefined}>
        <summary>Private certificate authority</summary>
        <label htmlFor="remote-ca">Trusted public CA certificate (PEM)</label>
        <textarea id="remote-ca" rows={5} value={caPem} onChange={event => setCaPem(event.target.value)} maxLength={16384} disabled={busy} autoCapitalize="none" autoCorrect="off" spellCheck={false} placeholder="-----BEGIN CERTIFICATE-----" />
        <p className="hint">Public certificate only, obtained through a trusted channel. This connection trusts only that CA.</p>
      </details>
      <label className="check"><input type="checkbox" checked={remember} onChange={event => setRemember(event.target.checked)} /> Remember this server address and CA on this device</label>
      <button className="primary" type="submit" disabled={!native || busy}>{busy ? "Connecting…" : "Connect securely"}</button>
      {error && <p role="alert" className="error">{error}</p>}
      {!native && <p className="hint">Open the native application to connect.</p>}
      <div className="connect-footer">
        {hasSaved && <button type="button" className="link" onClick={forgetServer}>Forget saved server</button>}
        <button type="button" className="link" onClick={() => setMode("local")}>Use a local development service</button>
      </div>
    </form>
  </div>;

  const registered = environments?.filter(environment => environment.status === "registered").length ?? 0;
  const nav = <>
    <NavItem icon="servers" label="Environments" active={view === "environments" || view === "environment"} badge={environments ? String(registered) : undefined} onClick={() => setView("environments")} />
    <NavItem icon="plus" label="New environment" short="New" active={view === "create"} onClick={() => setView("create")} />
    <NavItem icon="terminal" label="Terminal" active={view === "terminal"} dot={sshTarget !== null} onClick={() => setView("terminal")} />
    <NavItem icon="activity" label="Activity" active={view === "activity"} onClick={() => setView("activity")} />
    <NavItem icon="settings" label="Settings" active={view === "settings"} onClick={() => setView("settings")} />
  </>;
  const server = <div className="server-chip" title={connection.endpoint}><span className="dot online" />{host(connection.endpoint)}</div>;
  const running = drawer && drawerHidden ? <button type="button" className="running-pill" onClick={() => setDrawerHidden(false)}><span className="dot busy" />Open action · VM {drawer.environment.providerReference.vmid}</button> : null;
  const titles: Record<View, [string, string]> = {
    environments: ["Environments", "Virtual machines registered on this control plane"],
    environment: [selected?.definitionName ?? "Environment", selected ? `VM ${selected.providerReference.vmid} · ${selected.providerReference.node}` : ""],
    create: ["New environment", "Define, review and create a VM from a configured template"],
    terminal: ["Terminal", "Verified SSH sessions to your guests"],
    activity: ["Activity", "Operations retained on the server, including after restarts"],
    settings: ["Settings", "Server, identity and application"],
  };
  const [title, subtitle] = titles[view];

  return <Shell nav={nav} server={server} title={title} subtitle={subtitle}
    back={view === "environment" ? () => setView("environments") : undefined}
    actions={view === "environments" ? <>{running}
      <button type="button" className="ghost" disabled={loading} onClick={() => void loadWorkspace()}><Icon name="refresh" />{loading ? "Refreshing…" : "Refresh"}</button>
      <button type="button" className="primary" onClick={() => setView("create")}><Icon name="plus" />New environment</button>
    </> : running}>
    {loadError && <p role="alert" className="error banner">{loadError}</p>}

    {view === "environments" && <EnvironmentList environments={environments} loading={loading} onOpen={openEnvironment} onCreate={() => setView("create")} />}

    {view === "environment" && (selected ? <div className="detail">
      <div className="detail-header">
        {selected.status === "deleted" && <span className="chip deleted" title="Deleted through VMAnyware. The definition and history are kept and it can be recreated.">Deleted</span>}
        {selected.status === "registered" && <span className={`chip power ${power?.id === selected.id ? power.status : "unknown"}`}>{power?.id === selected.id ? power.status : "power unknown"}</span>}
        <dl className="facts"><div><dt>VMID</dt><dd>{selected.providerReference.vmid}</dd></div><div><dt>Node</dt><dd>{selected.providerReference.node}</dd></div><div><dt>Instance</dt><dd className="mono">{selected.id.slice(0, 12)}</dd></div></dl>
        <div className="toolbar">
          {selected.status === "registered" ? <>
            <button type="button" disabled={blocked || (power?.id === selected.id && power.status === "running")} title={power?.id === selected.id && power.status === "running" ? "Already running" : undefined} onClick={() => openDrawer({ kind: "lifecycle", environment: selected, action: "start" })}><Icon name="play" />Start</button>
            <button type="button" disabled={blocked || (power?.id === selected.id && power.status === "stopped")} title={power?.id === selected.id && power.status === "stopped" ? "Already stopped" : undefined} onClick={() => openDrawer({ kind: "lifecycle", environment: selected, action: "shutdown" })}><Icon name="power" />Shut down</button>
            <button type="button" disabled={blocked} onClick={() => openDrawer({ kind: "snapshot", environment: selected })}><Icon name="camera" />Snapshot</button>
            <button type="button" disabled={blocked} onClick={() => sshForInstance(selected)}><Icon name="terminal" />Connect</button>
            <button type="button" className="danger" disabled={blocked} onClick={() => openDrawer({ kind: "deletion", environment: selected })}><Icon name="trash" />Delete</button>
          </> : <button type="button" className="primary" disabled={blocked} onClick={() => openDrawer({ kind: "recreation", environment: selected })}><Icon name="repeat" />Recreate</button>}
        </div>
      </div>
      <div className="tabs" role="tablist">
        {(["overview", "snapshots", "history"] as Tab[]).filter(value => value !== "snapshots" || selected.status === "registered").map(value =>
          <button key={value} type="button" role="tab" aria-selected={tab === value} onClick={() => setTab(value)}>{value[0].toUpperCase() + value.slice(1)}</button>)}
      </div>
      <div className="tab-panel" role="tabpanel">
        {/* Kept mounted so the header power state stays current on other tabs. */}
        {<div className="grid-2" hidden={tab !== "overview"}>
          {selected.status === "registered" && <Panel title="Power"><RemoteInstanceStatus key={`power.${selected.id}.${revision}`} connectionId={connection.connectionId} environment={selected} disabled={busy || loading} invalidated={mutationLocked} autoRead onObserved={status => setPower(status ? { id: selected.id, status } : null)} /></Panel>}
          <Panel title="Definition"><RemoteInstanceDetail key={`detail.${selected.id}.${selected.status}`} connectionId={connection.connectionId} environment={selected} disabled={busy || loading} autoRead /></Panel>
        </div>}
        {tab === "snapshots" && selected.status === "registered" && <Panel title="Snapshots" action={<button type="button" disabled={blocked} onClick={() => openDrawer({ kind: "snapshot", environment: selected })}><Icon name="camera" />New snapshot</button>}>
          <RemoteSnapshots key={`snapshots.${selected.id}.${revision}`} connectionId={connection.connectionId} environment={selected} disabled={busy || loading} restoreDisabled={mutationLocked} autoLoad showRecovery={false} onRestore={snapshotName => openDrawer({ kind: "restore", environment: selected, snapshotName })} />
        </Panel>}
        {tab === "history" && <div className="stack">
          <Panel title="Start and shutdown"><RemoteLifecycleRecovery connectionId={connection.connectionId} environment={selected} disabled={busy || loading} /></Panel>
          {selected.status === "registered" && <Panel title="Snapshots and restores">
            <RemoteSnapshotRecovery connectionId={connection.connectionId} environment={selected} disabled={busy || loading} />
            <RemoteRestoreRecovery connectionId={connection.connectionId} environment={selected} disabled={busy || loading} />
          </Panel>}
          <Panel title="Deletion"><RemoteDeletionRecovery connectionId={connection.connectionId} environment={selected} disabled={busy || loading} /></Panel>
          {selected.status === "deleted" && <Panel title="Recreations"><RemoteRecreationRecovery connectionId={connection.connectionId} source={selected} disabled={busy || loading} /></Panel>}
        </div>}
      </div>
    </div> : <Empty title="Environment unavailable" text="It is no longer listed on this server." action={<button type="button" onClick={() => setView("environments")}>Back to environments</button>} />)}

    <div hidden={view !== "create"}>
      <Panel><RemoteCreate key={connection.connectionId} connectionId={connection.connectionId} endpoint={connection.endpoint} disabled={busy || mutationLocked} onSshTarget={selectSshTarget} onEnvironmentsChanged={reloadEnvironments} images={images?.map(image => image.image)} /></Panel>
    </div>

    {/* Kept mounted while hidden so a live SSH session survives navigation. */}
    <div hidden={view !== "terminal"}>
      {sshTarget?.instance && <p className="notice">Connecting to {sshTarget.instance.definitionName} · VM {sshTarget.instance.providerReference.vmid}. Enter the guest address yourself: power state and the saved definition do not reveal or verify it.</p>}
      <Panel><SshHostTrust key={sshTarget?.generation ?? 0} native={native} initialHost={sshTarget?.host ?? ""} initialUsername={sshTarget?.username ?? ""} /></Panel>
    </div>

    {view === "activity" && <Panel title="Creations"><RemoteRecovery key={`recovery-${connection.connectionId}`} connectionId={connection.connectionId} endpoint={connection.endpoint} disabled={busy || mutationLocked} onSshTarget={selectSshTarget} /></Panel>}

    {view === "settings" && <div className="stack">
      <Panel title="Server" action={<button type="button" className="danger" disabled={busy} onClick={() => void disconnect()}>Disconnect</button>}>
        <dl className="facts"><div><dt>Address</dt><dd className="mono">{connection.endpoint}</dd></div><div><dt>Session</dt><dd>Token held in memory only</dd></div></dl>
        <h3>Configured templates</h3>
        {images === null ? <p className="muted">Not loaded.</p> : images.length === 0 ? <p className="muted">No templates configured on this server.</p> :
          <div className="table">{images.map(image => <div className="row" key={image.image}><strong>{image.image}</strong><span>{image.node}</span><span>template {image.templateVmid}</span><span>{image.sourceDiskGiB} GiB</span><span className="mono">{image.templateVersion}</span></div>)}</div>}
        {hasSaved && <button type="button" className="link" onClick={forgetServer}>Forget saved server on this device</button>}
      </Panel>
      <Panel title="Application SSH identity"><AppIdentity disabled={busy} /></Panel>
      <Panel title="About">
        <dl className="facts"><div><dt>Application</dt><dd>VMAnyware · {native ? "native" : "browser preview"}</dd></div><div><dt>Platform</dt><dd>{runtime || "—"}</dd></div></dl>
      </Panel>
    </div>}

    {drawer && <div className="drawer-backdrop" role="presentation" hidden={drawerHidden} onClick={event => { if (event.target === event.currentTarget) closeDrawer(); }}>
      <aside className="drawer" role="dialog" aria-modal="true" aria-label="Review action">
        <header>
          <span className="drawer-target">{drawer.environment.definitionName} · VM {drawer.environment.providerReference.vmid}</span>
          <button type="button" className="icon" aria-label="Close" title={mutationLocked ? "Close; the action keeps running" : "Close"} onClick={closeDrawer}><Icon name="close" /></button>
        </header>
        <div className="drawer-body">
          {drawer.kind === "lifecycle" && <RemoteLifecycle key={`${drawer.environment.id}.${drawer.action}`} connectionId={connection.connectionId} environment={drawer.environment} action={drawer.action} disabled={busy || loading} onLock={setMutationLocked} onDone={finishDrawer} />}
          {drawer.kind === "snapshot" && <RemoteSnapshotCreate key={drawer.environment.id} connectionId={connection.connectionId} environment={drawer.environment} disabled={busy || loading} onLock={setMutationLocked} onDone={finishDrawer} />}
          {drawer.kind === "restore" && <RemoteSnapshotRestore key={`${drawer.environment.id}.${drawer.snapshotName}`} connectionId={connection.connectionId} environment={drawer.environment} snapshotName={drawer.snapshotName} disabled={busy || loading} onLock={setMutationLocked} onDone={finishDrawer} />}
          {drawer.kind === "deletion" && <RemoteDeletion key={drawer.environment.id} connectionId={connection.connectionId} environment={drawer.environment} disabled={busy || loading} onLock={setMutationLocked} onDone={finishDrawer} onDeleted={instanceId => setEnvironments(current => current?.map(environment => environment.id === instanceId ? { ...environment, status: "deleted" } : environment) ?? null)} />}
          {drawer.kind === "recreation" && <RemoteRecreation key={drawer.environment.id} connectionId={connection.connectionId} source={drawer.environment} disabled={busy || loading} onLock={setMutationLocked} onDone={finishDrawer} onSshTarget={(target, username) => { setDrawer(null); selectSshTarget(target, username); }} onEnvironmentsChanged={reloadEnvironments} />}
        </div>
      </aside>
    </div>}
  </Shell>;
}

function Shell({ nav, server, title, subtitle, back, actions, children }: { nav: ReactNode; server?: ReactNode; title: string; subtitle: string; back?: () => void; actions?: ReactNode; children: ReactNode }) {
  return <div className={nav ? "app" : "app no-nav"}>
    {nav && <aside className="sidebar" aria-label="Main navigation">
      <div className="brand"><span className="brand-mark" aria-hidden="true">›_</span>VMAnyware</div>
      <nav>{nav}</nav>
      <div className="sidebar-footer">{server}</div>
    </aside>}
    <div className="main">
      <header className="topbar">
        <div className="title">
          {back && <button type="button" className="icon" aria-label="Back" onClick={back}><Icon name="back" /></button>}
          <div><h1 id="workspace-title" tabIndex={-1}>{title}</h1>{subtitle && <p>{subtitle}</p>}</div>
        </div>
        <div className="topbar-actions">{actions}<span className="mobile-only">{server}</span></div>
      </header>
      <main className="content">{children}</main>
    </div>
    {nav && <nav className="tabbar" aria-label="Main navigation">{nav}</nav>}
  </div>;
}

function NavItem({ icon, label, short, active, badge, dot, onClick }: { icon: IconName; label: string; short?: string; active: boolean; badge?: string; dot?: boolean; onClick: () => void }) {
  return <button type="button" className="nav-item" aria-current={active ? "page" : undefined} onClick={onClick}>
    <Icon name={icon} size={18} /><span className="nav-label">{label}</span>{short && <span className="nav-short">{short}</span>}
    {badge !== undefined && <span className="nav-badge">{badge}</span>}
    {dot && <span className="dot online" aria-label="active" />}
  </button>;
}

function Panel({ title, action, children }: { title?: string; action?: ReactNode; children: ReactNode }) {
  return <section className="panel">
    {(title || action) && <header className="panel-header">{title && <h2>{title}</h2>}{action}</header>}
    <div className="panel-body">{children}</div>
  </section>;
}

function Empty({ title, text, action }: { title: string; text: string; action?: ReactNode }) {
  return <div className="empty"><h2>{title}</h2><p>{text}</p>{action}</div>;
}

function EnvironmentList({ environments, loading, onOpen, onCreate }: { environments: Environment[] | null; loading: boolean; onOpen: (environment: Environment) => void; onCreate: () => void }) {
  if (environments === null) return <div className="empty"><p>{loading ? "Loading environments…" : "Environments are not loaded."}</p></div>;
  if (environments.length === 0) return <Empty title="No environments yet" text="Create your first VM from a configured template." action={<button type="button" className="primary" onClick={onCreate}><Icon name="plus" />New environment</button>} />;
  const sorted = [...environments].sort((a, b) => Number(a.status === "deleted") - Number(b.status === "deleted") || a.providerReference.vmid - b.providerReference.vmid);
  return <div className="list" role="list">
    <div className="list-head" aria-hidden="true"><span>Name</span><span>VMID</span><span>Node</span><span>Status</span><span /></div>
    {sorted.map(environment => <button type="button" role="listitem" key={environment.id} className={`list-row ${environment.status}`} onClick={() => onOpen(environment)}>
      <span className="name"><Icon name="servers" />{environment.definitionName}</span>
      <span className="mono">{environment.providerReference.vmid}</span>
      <span>{environment.providerReference.node}</span>
      <span><span className={`chip ${environment.status}`} title={environment.status === "registered" ? "A VM managed by this control plane" : "Deleted; definition and history kept, can be recreated"}>{environment.status === "registered" ? "Active" : "Deleted"}</span></span>
      <span className="chevron"><Icon name="chevron" /></span>
    </button>)}
  </div>;
}
