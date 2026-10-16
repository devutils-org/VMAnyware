import { StrictMode, useState, type FormEvent } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import "./styles.css";
import sampleDefinition from "../../../examples/kernel-lab.yaml?raw";

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

function App() {
  const [status, setStatus] = useState("Not checked");
  const [checking, setChecking] = useState(false);
  const native = isTauri();
  const [port, setPort] = useState("8080");
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const [connectionError, setConnectionError] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [yaml, setYaml] = useState(sampleDefinition);
  const [definitions, setDefinitions] = useState<DefinitionRecord[]>([]);
  const [definitionBusy, setDefinitionBusy] = useState(false);
  const [definitionMessage, setDefinitionMessage] = useState("");
  const [definitionError, setDefinitionError] = useState("");

  async function definitionsAction(save: boolean) {
    const value = Number(port);
    if (!Number.isInteger(value) || value < 1 || value > 65535) {
      setDefinitionError("Enter a port between 1 and 65535.");
      return;
    }
    setDefinitionBusy(true);
    setDefinitionError("");
    setDefinitionMessage("");
    try {
      if (save) {
        const record = await invoke<DefinitionRecord>("save_definition", { port: value, yaml });
        setYaml(record.yaml);
        setDefinitionMessage(`Saved ${record.name}. No VM was created.`);
      }
      setDefinitions(await invoke<DefinitionRecord[]>("list_definitions", { port: value }));
      if (!save) setDefinitionMessage("Saved definitions loaded.");
    } catch (error) {
      setDefinitionError(typeof error === "string" ? error : "Cannot load or save definitions. Please retry.");
    } finally {
      setDefinitionBusy(false);
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
    try {
      setInventory(await invoke<Inventory>("inspect_control_plane", { port: value }));
    } catch (error) {
      setConnectionError(typeof error === "string" ? error : "Connection failed. Please retry.");
    } finally {
      setConnecting(false);
    }
  }

  async function checkRuntime() {
    setChecking(true);
    try {
      const platform = await invoke<string>("runtime_platform");
      setStatus(`Native bridge connected (${platform})`);
    } catch {
      setStatus("Native bridge check failed. Retry from the desktop app.");
    } finally {
      setChecking(false);
    }
  }

  return (
    <main>
      <p className="eyebrow">PocketLab · Phase 0</p>
      <h1>Your systems lab, within reach.</h1>
      <p>Connect to your local control plane to inspect Proxmox nodes and available templates.</p>
      <section aria-labelledby="connection-heading" aria-busy={connecting}>
        <h2 id="connection-heading">Control plane</h2>
        <p>This prototype connects to the service on this computer. Proxmox credentials stay on the server.</p>
        <form onSubmit={connect}>
          <label htmlFor="port">Local service port</label>
          <div className="connection-fields">
            <input id="port" type="number" min="1" max="65535" required value={port}
              disabled={connecting || definitionBusy} onChange={(event) => {
                setPort(event.target.value);
                setInventory(null);
                setConnectionError("");
                setDefinitions([]);
                setDefinitionMessage("");
                setDefinitionError("");
              }} />
            <button disabled={!native || connecting} type="submit">
              {connecting ? "Reading inventory…" : inventory ? "Refresh inventory" : "Connect"}
            </button>
          </div>
        </form>
        {!native && <p>Open the native application to connect to the local service.</p>}
        {connectionError && <p role="alert" className="error">{connectionError}</p>}
        {inventory && <div role="status"><p>Connected · Proxmox {inventory.version}</p></div>}
      </section>
      <section aria-labelledby="definitions-heading" aria-busy={definitionBusy}>
        <h2 id="definitions-heading">Environment definitions</h2>
        <p>Save a reusable configuration before provisioning. Replace the example public key with your own. Saving does not create a VM.</p>
        <label htmlFor="definition-yaml">Environment YAML</label>
        <textarea id="definition-yaml" value={yaml} spellCheck={false} disabled={definitionBusy}
          onChange={(event) => { setYaml(event.target.value); setDefinitionMessage(""); setDefinitionError(""); }} />
        <div className="connection-fields">
          <button disabled={!native || definitionBusy} onClick={() => definitionsAction(true)}>
            {definitionBusy ? "Working…" : "Save definition"}
          </button>
          <button disabled={!native || definitionBusy} onClick={() => definitionsAction(false)}>Load saved definitions</button>
        </div>
        <p>CPU: 1–64 · Memory: 256–262144 MiB · Disk: 4–4096 GiB</p>
        {definitionMessage && <p role="status">{definitionMessage}</p>}
        {definitionError && <p role="alert" className="error">{definitionError}</p>}
        {definitions.length > 0 && <ul>{definitions.map(record => <li key={record.name}>
          <button disabled={definitionBusy} onClick={() => { setYaml(record.yaml); setDefinitionMessage(`Loaded ${record.name}. Change its name to save a new definition.`); setDefinitionError(""); }}>
            {record.name}
          </button> · Revision {record.revision}
        </li>)}</ul>}
      </section>
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
          <p>Inventory is read-only. Creating environments is not available yet.</p>
        </section>
      </>}
      <section aria-labelledby="runtime-heading">
        <h2 id="runtime-heading">Application runtime</h2>
        <p>{native ? "Native application" : "Browser preview"}</p>
        <button disabled={!native || checking} onClick={checkRuntime}>
          {checking ? "Checking…" : "Check native bridge"}
        </button>
        <p role="status">{native ? status : "Open the native application to check its connection to Rust."}</p>
      </section>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode><App /></StrictMode>,
);
