// Deliberately simulated native IPC. Never invokes Rust or discovers an agent.
(() => {
  const calls = [];
  let reads = 0, starts = 0, lateStart;
  const pendingReads = [];
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = { invoke: async (command, args = {}) => {
    calls.push({ command, args });
    const endpoint = { host: "fixture.local", port: 22 };
    switch (command) {
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
      await wait(() => document.querySelector(".xterm-rows")?.textContent.includes("PocketLab fixture €"));
      await wait(() => count("resize_ssh_session") > 0);
      check(starts === 1, "StrictMode started duplicate native connections");
      check(count("close_ssh_session") === 0, "StrictMode prematurely closed the session");
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
      window.webkit.messageHandlers.result.postMessage({ ok: true, message: "WebKit development smoke passed: explicit selection, split UTF-8 rendering, resize, Unicode input, Ctrl-C, StrictMode lifecycle, disconnect and stale-start cleanup (fixture IPC only)." });
    } catch (error) {
      window.webkit.messageHandlers.result.postMessage({ ok: false, message: String(error) });
    }
  });
})();
