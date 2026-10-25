import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import sampleDefinition from "../../../examples/kernel-lab.yaml?raw";
import { SshHostTrust } from "./SshHostTrust";
import { ProvisioningPreview } from "./ProvisioningPreview";
import { OperationHistory } from "./OperationHistory";
import { Instances } from "./Instances";
import { AppIdentity } from "./AppIdentity";

// Developer workflow against a control plane on this computer (loopback only).
type DefinitionRecord = {
  name: string;
  yaml: string;
  hash: string;
  revision: number;
  createdAt: string;
  updatedAt: string;
};

type Inventory = {
  provider: string;
  version: string;
  nodes: { name: string; status: string }[];
  templates: { node: string; vmid: number; name: string | null }[];
};

type Operation = {
  id: string;
  kind: string;
  status: "queued" | "running" | "succeeded" | "failed" | "interrupted";
  result: Inventory | null;
  failure: string | null;
};

const failureMessages: Record<string, string> = {
  provider_not_configured: "Configure Proxmox on the control plane before reading inventory.",
  provider_unauthorized: "Proxmox rejected the configured API token.",
  provider_forbidden: "The Proxmox token lacks inventory permissions.",
  provider_timeout: "Proxmox inventory timed out. Check the connection and retry.",
  provider_unavailable: "Cannot read Proxmox inventory. Check the endpoint and TLS trust.",
  process_interrupted: "The service stopped during this refresh. Its outcome is unconfirmed; retry to start a new read-only refresh.",
};

function savedRequestKey(key: string): string | null {
  try { return localStorage.getItem(key); } catch { return null; }
}

function rememberRequestKey(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch { /* Resume caching is optional; backend operation records remain durable. */ }
}

export function LocalWorkspace({ native }: { native: boolean }) {
  const [port, setPort] = useState("8080");
  const [serviceState, setServiceState] = useState<"unchecked" | "checking" | "ready" | "unavailable">("unchecked");
  const [serviceError, setServiceError] = useState("");
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const [connectionError, setConnectionError] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [operation, setOperation] = useState<Operation | null>(null);
  const [yaml, setYaml] = useState(sampleDefinition);
  const [definitions, setDefinitions] = useState<DefinitionRecord[]>([]);
  const [definitionBusy, setDefinitionBusy] = useState(false);
  const [creationLocked, setCreationLocked] = useState(false);
  const [definitionMessage, setDefinitionMessage] = useState("");
  const [definitionError, setDefinitionError] = useState("");

  async function definitionsAction(action: "save" | "load" | "validate") {
    if (creationLocked) return;
    const value = Number(port);
    if (!Number.isInteger(value) || value < 1 || value > 65535) {
      setDefinitionError("Enter a port between 1 and 65535.");
      return;
    }
    setDefinitionBusy(true);
    setDefinitionError("");
    setDefinitionMessage("");
    try {
      if (action === "validate") {
        const result = await invoke<{ name: string; hash: string; yaml: string }>("validate_definition", { port: value, yaml });
        setDefinitionMessage(`Validated ${result.name}. Nothing was saved or provisioned.`);
        return;
      }
      if (action === "save") {
        const record = await invoke<DefinitionRecord>("save_definition", { port: value, yaml });
        setYaml(record.yaml);
        setDefinitionMessage(`Saved ${record.name}. No VM was created.`);
      }
      setDefinitions(await invoke<DefinitionRecord[]>("list_definitions", { port: value }));
      if (action === "load") setDefinitionMessage("Saved definitions loaded.");
    } catch (error) {
      setDefinitionError(typeof error === "string" ? error : "Cannot load or save definitions. Please retry.");
    } finally {
      setDefinitionBusy(false);
    }
  }

  async function checkService() {
    const value = Number(port);
    if (!Number.isInteger(value) || value < 1 || value > 65535) {
      setServiceState("unavailable");
      setServiceError("Enter a port between 1 and 65535.");
      return;
    }
    setServiceState("checking");
    setServiceError("");
    try {
      await invoke("check_control_plane", { port: value });
      setServiceState("ready");
    } catch (error) {
      setServiceState("unavailable");
      setServiceError(typeof error === "string" ? error : "Cannot reach the local service. Check the port and retry.");
    }
  }

  async function connect(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const value = Number(port);
    if (!Number.isInteger(value) || value < 1 || value > 65535) {
      setConnectionError("Enter a port between 1 and 65535.");
      return;
    }
    setConnecting(true);
    setInventory(null);
    setConnectionError("");
    const cacheKey = `vmanyware.inventory.request.${value}`;
    try {
      const requestKey = savedRequestKey(cacheKey) ?? crypto.randomUUID();
      rememberRequestKey(cacheKey, requestKey);
      let current = await invoke<Operation>("begin_inventory_refresh", { port: value, requestKey });
      setOperation(current);
      const deadline = Date.now() + 60_000;
      while (current.status === "queued" || current.status === "running") {
        if (Date.now() >= deadline) throw "Monitoring paused. Connect again to resume the same operation.";
        await new Promise(resolve => setTimeout(resolve, 400));
        current = await invoke<Operation>("read_operation", { port: value, id: current.id });
        setOperation(current);
      }
      rememberRequestKey(cacheKey, null);
      if (current.status === "succeeded" && current.result) setInventory(current.result);
      else setConnectionError(failureMessages[current.failure ?? ""] ?? "The inventory operation did not return a result. Please retry.");
    } catch (error) {
      setConnectionError(typeof error === "string" ? error : "Connection failed. Please retry.");
    } finally {
      setConnecting(false);
    }
  }


  return <div className="local-workspace">
      <div className="workspace-summary" aria-label="Current workspace observations">
        <div className="summary-card"><span className="summary-label">Local service</span><strong className={`service-indicator ${serviceState}`}>{serviceState === "ready" ? "Available" : serviceState === "checking" ? "Checking…" : serviceState === "unavailable" ? "Unavailable" : "Not checked"}</strong><span>Port {port || "—"} · loopback</span></div>
        <div className="summary-card"><span className="summary-label">Proxmox nodes</span><strong>{inventory ? inventory.nodes.length : "—"}</strong><span>{inventory ? `Last inventory · v${inventory.version}` : "Connect to read inventory"}</span></div>
        <div className="summary-card"><span className="summary-label">Visible templates</span><strong>{inventory ? inventory.templates.length : "—"}</strong><span>{inventory ? "Visible to the configured token" : "No inventory loaded"}</span></div>
      </div>
      <section aria-labelledby="connection-heading" aria-busy={connecting}>
        <h2 id="connection-heading" tabIndex={-1}>Local service</h2>
        <p>Connect to the service on this computer. Proxmox credentials stay on the server.</p>
        <form onSubmit={connect}>
          <label htmlFor="port">Local service port</label>
          <div className="connection-fields">
            <input id="port" type="number" min="1" max="65535" required value={port}
              disabled={connecting || definitionBusy || creationLocked || serviceState === "checking"} onChange={(event) => {
                setPort(event.target.value);
                setServiceState("unchecked");
                setServiceError("");
                setInventory(null);
                setConnectionError("");
                setOperation(null);
                setDefinitions([]);
                setDefinitionMessage("");
                setDefinitionError("");
              }} />
            <button type="button" disabled={!native || serviceState === "checking"} onClick={checkService}>
              {serviceState === "checking" ? "Checking service…" : serviceState === "unavailable" ? "Retry service check" : "Check local service"}
            </button>
            <button disabled={!native || connecting} type="submit">
              {connecting ? "Reading inventory…" : inventory ? "Refresh inventory" : "Connect"}
            </button>
          </div>
        </form>
        <p id="service-status" role="status">
          {serviceState === "unchecked" ? "Local service not checked." :
            serviceState === "checking" ? "Checking the local service…" :
            serviceState === "ready" ? "Local service available. Definitions and provisioning previews can be used without Proxmox." :
            "Local service unavailable."}
        </p>
        {serviceError && <p role="alert" className="error">{serviceError}</p>}
        <p>Check the local service first, then use Connect to read Proxmox inventory once the provider is configured.</p>
        {!native && <p>Open the native application to connect to the local service.</p>}
        {connectionError && <p role="alert" className="error">{connectionError}</p>}
        {operation && <p role="status">Inventory refresh: {operation.status} · <span className="operation-id">{operation.id}</span></p>}
        {inventory && <div role="status"><p>Connected · Proxmox {inventory.version}</p></div>}
      </section>
      <Instances key={`instances-${port}`} port={port} native={native} />
      <OperationHistory key={port} port={port} native={native} />
      <div className="definition-workspace">
      <section aria-labelledby="definitions-heading" aria-busy={definitionBusy}>
        <p className="panel-kicker">02 / Define</p>
        <h2 id="definitions-heading" tabIndex={-1}>Environment definitions</h2>
        <p>Save a reusable configuration before provisioning. Replace the example public key with your own. Saving does not create a VM.</p>
        {native && <AppIdentity disabled={definitionBusy || creationLocked} />}
        <label htmlFor="definition-yaml">Environment YAML</label>
        <textarea id="definition-yaml" value={yaml} spellCheck={false} disabled={definitionBusy || creationLocked}
          onChange={(event) => { setYaml(event.target.value); setDefinitionMessage(""); setDefinitionError(""); }} />
        <div className="connection-fields">
          <button disabled={!native || definitionBusy || creationLocked} onClick={() => definitionsAction("validate")}>Validate YAML</button>
          <button className="definition-save" disabled={!native || definitionBusy || creationLocked} onClick={() => definitionsAction("save")}>
            {definitionBusy ? "Working…" : "Save definition"}
          </button>
          <button disabled={!native || definitionBusy || creationLocked} onClick={() => definitionsAction("load")}>Load saved definitions</button>
        </div>
        <p>CPU: 1–64 · Memory: 256–262144 MiB · Disk: 4–4096 GiB</p>
        {definitionMessage && <p role="status">{definitionMessage}</p>}
        {definitionError && <p role="alert" className="error">{definitionError}</p>}
        {definitions.length > 0 && <ul>{definitions.map(record => <li key={record.name}>
          <button disabled={definitionBusy || creationLocked} onClick={() => { setYaml(record.yaml); setDefinitionMessage(`Loaded ${record.name}. Change its name to save a new definition.`); setDefinitionError(""); }}>
            {record.name}
          </button> · Revision {record.revision}
        </li>)}</ul>}
      </section>
      <ProvisioningPreview port={port} yaml={yaml} native={native} onCreationLockedChange={setCreationLocked} />
      </div>
      <SshHostTrust native={native} />
      {inventory && <>
        <section aria-labelledby="nodes-heading">
          <h2 id="nodes-heading">Nodes</h2>
          {inventory.nodes.length === 0 ? <p>No nodes are visible to this token.</p> :
            <ul>{inventory.nodes.map(node => <li key={node.name}><strong>{node.name}</strong> · {node.status}</li>)}</ul>}
        </section>
        <section aria-labelledby="templates-heading">
          <h2 id="templates-heading">Templates</h2>
          {inventory.templates.length === 0 ? <p>No templates are visible on online nodes. Check template preparation and token permissions.</p> :
            <ul>{inventory.templates.map(template => <li key={`${template.node}:${template.vmid}`}>
              <strong>{template.name ?? "Unnamed template"}</strong> · {template.node} · VMID {template.vmid}
            </li>)}</ul>}
          <p>Inventory is read-only. Use the reviewed provisioning plan to create an environment.</p>
        </section>
      </>}
    </div>;
}
