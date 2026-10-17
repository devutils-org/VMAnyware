// Deliberately simulated native IPC. Never invokes Rust or discovers an agent.
(() => {
  const calls = [];
  let reads = 0, starts = 0, healthChecks = 0, refreshes = 0, validations = 0, lateStart;
  const pendingReads = [];
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = { invoke: async (command, args = {}) => {
    calls.push({ command, args });
    const endpoint = { host: "fixture.local", port: 22 };
    switch (command) {
      case "check_control_plane":
        if (++healthChecks === 1) throw "Cannot reach the local control plane. Start it with make server and check the port.";
        return;
      case "begin_inventory_refresh":
        refreshes++;
        return { id: `inventory-${refreshes}`, status: "queued", result: null, failure: null };
      case "read_operation":
        return { id: args.id, status: refreshes === 2 ? "failed" : "succeeded",
          failure: refreshes === 2 ? "provider_forbidden" : null,
          result: refreshes === 2 ? null : { provider: "proxmox", version: "fixture-version",
            nodes: refreshes === 1 ? [{ name: "fixture-node", status: "online" }] : [],
            templates: refreshes === 1 ? [{ node: "fixture-node", vmid: 9000, name: "fixture-template" }] : [] } };
      case "validate_definition":
        if (++validations === 1) throw "Fixture validation error: replace the example public key.";
        return { name: "fixture-definition", hash: "a".repeat(64), yaml: args.yaml };
      case "preview_provisioning":
        return { definitionName: "fixture-definition", definitionHash: "a".repeat(64), planHash: "b".repeat(64),
          source: { image: "fixture-debian", node: "fixture-node", templateVmid: 9000, templateVersion: "fixture-version", sourceDiskGiB: 4, diskDevice: "scsi0" },
          target: { node: "fixture-node", vmid: args.targetVmid, name: "fixture-definition", storage: "fixture-storage", bridge: "fixture-bridge", guestUser: "fixture", ipv4: "dhcp", resources: { cpu: 2, memoryMiB: 2048, diskGiB: 16 } },
          sshKeyCount: 1, packages: ["git"], snapshotAfterProvisioning: false, liveVerified: false, vmidReserved: false, creationEnabled: false };
      case "preflight_provisioning": return { planHash: "b".repeat(64), creationEnabled: false, vmidReserved: false,
        checks: [{ code: "template", status: "passed", message: "Fixture template is readable." },
          { code: "storage", status: "failed", message: "Fixture storage has insufficient capacity." },
          { code: "network", status: "unknown", message: "Fixture network requires verification." }] };
      case "ssh_host_support": return true;
      case "inspect_ssh_host": return { ...endpoint, algorithm: "ssh-ed25519", fingerprint: "SHA256:fixture-host", status: "trusted", challengeId: null, expiresInSeconds: 0 };
      case "list_ssh_identities": return { ...endpoint, username: args.username, id: `selection-${starts}`, identities: [{ algorithm: "ssh-ed25519", fingerprint: "SHA256:fixture-user" }], expiresInSeconds: 120 };
      case "start_ssh_session":
        if (++starts === 1) return { id: "session-one" };
        return new Promise(resolve => { lateStart = resolve; });
      case "read_ssh_session":
        if (++reads === 1) return { type: "data", bytes: [...new TextEncoder().encode("PocketLab fixture "), 0xe2, 0x82] };
        if (reads === 2) return { type: "data", bytes: [0xac, 13, 10] };
        return new Promise(resolve => pendingReads.push(resolve));
      case "close_ssh_session": pendingReads.splice(0).forEach(resolve => resolve({ type: "closed" })); return;
      case "write_ssh_session": case "resize_ssh_session": case "cancel_ssh_authentication": return;
      default: throw new Error(`Unexpected fixture IPC command: ${command}`);
    }
  } };
  const check = (condition, message) => { if (!condition) throw new Error(message); };
  const wait = async predicate => {
    const deadline = Date.now() + 5000;
    while (!predicate()) {
      if (Date.now() >= deadline) throw new Error("Timed out waiting for fixture UI");
      await new Promise(resolve => setTimeout(resolve, 10));
    }
  };
  const button = text => [...document.querySelectorAll("button")].find(button => button.textContent === text);
  const input = (id, value) => {
    const node = document.getElementById(id);
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(node, value);
    node.dispatchEvent(new Event("input", { bubbles: true }));
  };
  const count = command => calls.filter(call => call.command === command).length;
  window.addEventListener("DOMContentLoaded", async () => {
    try {
      await wait(() => button("Inspect host key") && !button("Inspect host key").disabled);
      check(count("list_ssh_identities") === 0, "Startup accessed agent");
      check(healthChecks === 0, "Startup automatically checked the service");
      button("Check local service").click();
      await wait(() => button("Retry service check"));
      check(document.getElementById("service-status").textContent.includes("unavailable"), "Missing service failure status");
      check(document.querySelector('[role="alert"]').textContent.includes("make server"), "Missing recovery hint");
      button("Retry service check").click();
      await wait(() => document.getElementById("service-status").textContent.includes("available."));
      check(!document.querySelector('[role="alert"]'), "Retry retained stale error");
      check(count("begin_inventory_refresh") === 0, "Health check requested Proxmox inventory");
      input("port", "8081");
      await wait(() => document.getElementById("service-status").textContent.includes("not checked"));
      input("port", "8080");
      check(document.querySelector(".service-indicator").textContent === "Not checked", "Summary did not reset service status");
      const summaryValues = () => [...document.querySelectorAll(".summary-card strong")].map(node => node.textContent);
      check(summaryValues().slice(1).every(value => value === "—"), "Unloaded inventory displayed fabricated counts");
      button("Connect").click();
      await wait(() => document.getElementById("templates-heading"));
      check(summaryValues().slice(1).join(",") === "1,1", "Loaded inventory summary mismatch");
      check(document.body.textContent.includes("fixture-template"), "Loaded template missing");
      button("Refresh inventory").click();
      await wait(() => document.querySelector('[role="alert"]')?.textContent.includes("lacks inventory permissions"));
      check(summaryValues().slice(1).every(value => value === "—"), "Failed refresh retained stale inventory counts");
      button("Connect").click();
      await wait(() => document.getElementById("nodes-heading"));
      check(summaryValues().slice(1).join(",") === "0,0", "Empty inventory was not distinguished from unloaded inventory");
      check(document.body.textContent.includes("No nodes are visible"), "Empty node message missing");
      button("Validate YAML").click();
      await wait(() => document.querySelector('[role="alert"]')?.textContent.includes("Fixture validation error"));
      button("Validate YAML").click();
      await wait(() => document.body.textContent.includes("Validated fixture-definition"));
      check(!document.querySelector('[role="alert"]'), "Successful validation retained stale error");
      input("target-vmid", "9010"); button("Preview provisioning").click();
      await wait(() => button("Check against Proxmox"));
      button("Check against Proxmox").click();
      await wait(() => document.getElementById("preflight-heading"));
      const checks = document.getElementById("preflight-heading").parentElement.textContent;
      check(["Passed:", "Failed:", "Unknown:"].every(label => checks.includes(label)), "Mixed preflight states missing");
      check(checks.includes("creation remains unavailable"), "Preflight implied creation is available");
      check(document.documentElement.scrollWidth <= innerWidth, "Loaded provisioning result overflowed the viewport");
      input("port", "8081");
      await wait(() => !document.getElementById("preflight-heading"));
      check(summaryValues().slice(1).every(value => value === "—"), "Port edit retained inventory");
      input("port", "8080");
      input("ssh-host", "fixture.local"); button("Inspect host key").click();
      await wait(() => document.getElementById("ssh-username"));
      check(count("list_ssh_identities") === 0, "Host inspection accessed agent");
      input("ssh-username", "fixture");
      document.querySelector('label input[type="checkbox"]').click();
      await wait(() => !button("List agent identities").disabled);
      button("List agent identities").click();
      await wait(() => document.querySelector('input[name="ssh-identity"]'));
      check(button("Open terminal").disabled, "Identity was preselected");
      document.querySelector('input[name="ssh-identity"]').click();
      await wait(() => !button("Open terminal").disabled);
      button("Open terminal").click();
      if (new URLSearchParams(location.search).get("rendererFailure") === "1") {
        await wait(() => document.querySelector('[role="alert"]')?.textContent.includes("Cannot prepare or open"));
        check(starts === 0, "Renderer loading failure opened an SSH connection");
        check(count("cancel_ssh_authentication") > 0, "Failed renderer retained the selection");
        window.webkit.messageHandlers.result.postMessage({ ok: true, message: "WebKit renderer-load failure passed: visible error, selection cancelled, zero native starts (fixture IPC only)." });
        return;
      }
      const moduleDeadline = Date.now() + 5000;
      while (!(await (await fetch("/__terminal_fixture/pending")).json()).pending) {
        check(Date.now() < moduleDeadline, "Renderer module did not reach fixture gate");
        await new Promise(resolve => setTimeout(resolve, 10));
      }
      check(starts === 0, "SSH started before renderer finished loading");
      input("ssh-username", "changed-before-renderer");
      await fetch("/__terminal_fixture/release");
      await wait(() => globalThis.__terminalFixtureLoaded);
      await new Promise(resolve => setTimeout(resolve, 0));
      check(starts === 0, "Cancelled renderer load started a late SSH connection");
      input("ssh-username", "fixture");
      button("List agent identities").click();
      await wait(() => document.querySelector('input[name="ssh-identity"]'));
      document.querySelector('input[name="ssh-identity"]').click();
      await wait(() => !button("Open terminal").disabled);
      button("Open terminal").click();
      await wait(() => document.querySelector(".xterm-rows")?.textContent.includes("PocketLab fixture €"));
      await wait(() => count("resize_ssh_session") > 0);
      check(starts === 1, "StrictMode started duplicate native connections");
      check(count("close_ssh_session") === 0, "StrictMode prematurely closed the session");
      const mountedTerminal = document.querySelector(".ssh-terminal");
      for (const destination of ["definitions-heading", "plan-heading", "connection-heading", "ssh-trust-heading"]) {
        document.querySelector(`nav a[href="#${destination}"]`).click();
        await new Promise(resolve => setTimeout(resolve, 0));
        check(document.activeElement.id === destination, "Workspace destination did not receive focus");
        check(document.querySelector(".ssh-terminal") === mountedTerminal, "Navigation replaced the terminal");
      }
      check(starts === 1 && count("close_ssh_session") === 0, "Navigation restarted or closed the session");
      check(document.documentElement.scrollWidth <= innerWidth, "Mounted terminal overflowed the viewport");
      const keyboard = document.querySelector(".xterm-helper-textarea");
      keyboard.value = "λ";
      keyboard.dispatchEvent(new InputEvent("input", { inputType: "insertText", data: "λ", bubbles: true }));
      await wait(() => calls.some(call => call.command === "write_ssh_session" && JSON.stringify(call.args.bytes) === "[206,187]"));
      button("Send Ctrl-C").click();
      await wait(() => calls.some(call => call.command === "write_ssh_session" && call.args.bytes.length === 1 && call.args.bytes[0] === 3));
      button("Disconnect").click();
      await wait(() => count("close_ssh_session") === 1);
      // Cancel while another start is pending, then return its late native ID.
      button("List agent identities").click();
      await wait(() => document.querySelector('input[name="ssh-identity"]'));
      document.querySelector('input[name="ssh-identity"]').click();
      await wait(() => !button("Open terminal").disabled);
      button("Open terminal").click();
      await wait(() => !!lateStart);
      input("ssh-username", "changed");
      lateStart({ id: "session-late" });
      await wait(() => calls.some(call => call.command === "close_ssh_session" && call.args.sessionId === "session-late"));
      check(!document.querySelector(".ssh-terminal"), "Stale session mounted a terminal");
      window.webkit.messageHandlers.result.postMessage({ ok: true, message: "Renderer development smoke passed: workspace loaded/empty/error states, mixed preflight results, navigation with mounted terminal, cancelled module load, explicit selection, split UTF-8 rendering, resize, Unicode input, Ctrl-C, StrictMode lifecycle, disconnect and stale-start cleanup (fixture IPC only)." });
    } catch (error) {
      window.webkit.messageHandlers.result.postMessage({ ok: false, message: String(error) });
    }
  });
})();
