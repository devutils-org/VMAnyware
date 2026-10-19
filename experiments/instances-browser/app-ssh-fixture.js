// Real React/terminal renderer, simulated IPC, no external SSH or credentials.
(() => {
  const calls = [], pending = [];
  const passphrase = "disposable browser SSH phrase 42";
  let selections = 0, starts = 0, reads = 0, lateUnlock, lateStart;
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = { invoke: async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "ssh_host_support") return true;
    if (command === "ssh_agent_support") return false;
    if (command === "inspect_ssh_host") return {host:args.host,port:args.port,algorithm:"ssh-ed25519",fingerprint:"SHA256:fixture-host",status:args.host === "changed.test" ? "changed" : "trusted",challengeId:null,expiresInSeconds:0};
    if (command === "list_app_ssh_identity") {
      check(args.passphrase === passphrase && args.host === "fixture.test" && args.username === "fixture" && args.port === 22, "Wrong identity account or secret");
      const result = {host:args.host,port:args.port,username:args.username,id:`app-selection-${++selections}`,identities:[{algorithm:"ssh-ed25519",fingerprint:"SHA256:fixture-app"}],expiresInSeconds:120};
      if (selections === 1) throw "Cannot unlock application SSH identity";
      if (selections === 4) return new Promise(resolve => { lateUnlock = () => resolve(result); });
      return result;
    }
    if (command === "check_ssh_authentication") {
      check(args.fingerprint === "SHA256:fixture-app", "Wrong selected identity");
      return {host:"fixture.test",port:22,username:"fixture",fingerprint:args.fingerprint};
    }
    if (command === "start_ssh_session") {
      check(!("passphrase" in args) && args.fingerprint === "SHA256:fixture-app", "Session did not consume the selected identity");
      if (++starts === 2) return new Promise(resolve => { lateStart = () => resolve({id:"late-app-session"}); });
      return {id:"app-session"};
    }
    if (command === "read_ssh_session") {
      if (++reads === 1) return {type:"data",bytes:[...new TextEncoder().encode("fixture terminal ready\r\n")]};
      return new Promise(resolve => pending.push(resolve));
    }
    if (command === "close_ssh_session") { pending.splice(0).forEach(resolve => resolve({type:"closed"})); return; }
    if (["write_ssh_session","resize_ssh_session","cancel_ssh_authentication","cancel_ssh_host"].includes(command)) return;
    throw new Error(`Unexpected app SSH fixture command: ${command}`);
  }};
  const check = (value, message) => { if (!value) throw new Error(message); };
  const text = () => document.body?.textContent || "";
  const button = label => [...document.querySelectorAll("button")].find(element => element.textContent === label);
  const count = command => calls.filter(call => call.command === command).length;
  async function wait(condition) {
    const deadline = Date.now() + 5000;
    while (!condition()) { check(Date.now() < deadline, `UI wait timed out: ${text().slice(-300)}`); await new Promise(resolve => setTimeout(resolve, 10)); }
  }
  function input(id, value) {
    const element = document.getElementById(id);
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(element, value);
    element.dispatchEvent(new Event("input", {bubbles:true}));
  }
  async function inspect(host) {
    input("ssh-host", host); await wait(() => document.getElementById("ssh-host").value === host);
    button("Inspect host key").click();
    await wait(() => host === "changed.test" ? text().includes("This host key differs") : document.getElementById("ssh-username"));
  }
  async function unlock() {
    input("ssh-username", "fixture"); await wait(() => document.getElementById("ssh-username").value === "fixture");
    const consent = document.querySelector('#ssh-username').closest("form").querySelector('input[type="checkbox"]');
    if (!consent.checked) { consent.click(); await wait(() => !button("Unlock connection identity").disabled); }
    input("ssh-key-passphrase", passphrase); await wait(() => document.getElementById("ssh-key-passphrase").value === passphrase);
    button("Unlock connection identity").click();
  }
  async function choose() {
    await wait(() => document.querySelector('input[name="ssh-identity"]'));
    check(button("Open terminal").disabled, "Identity was selected without user choice");
    document.querySelector('input[name="ssh-identity"]').click();
    await wait(() => !button("Open terminal").disabled);
  }
  (async () => {
    await wait(() => button("Inspect host key") && !button("Inspect host key").disabled);
    check(count("list_app_ssh_identity") === 0, "Startup unlocked an identity");
    await inspect("changed.test");
    check(!document.getElementById("ssh-username"), "Changed host allowed authentication");
    await inspect("fixture.test");
    check(!document.querySelector('#ssh-source option[value="agent"]'), "Agent offered on this platform");
    await unlock(); await wait(() => text().includes("Cannot unlock application SSH identity"));
    check(document.getElementById("ssh-key-passphrase").value === "", "Failed unlock retained the passphrase");
    check(count("start_ssh_session") === 0, "Failed unlock opened a session");
    await unlock(); await choose(); button("Check selected identity").click();
    await wait(() => text().includes("Authenticated as fixture"));
    check(count("start_ssh_session") === 0, "Authentication check opened a terminal");
    await unlock(); await choose(); button("Open terminal").click();
    await wait(() => document.querySelector(".ssh-terminal .xterm-helper-textarea"));
    check(document.getElementById("ssh-key-passphrase").value === "", "Terminal retained passphrase field");
    button("Send Ctrl-C").click();
    await wait(() => calls.some(call => call.command === "write_ssh_session" && call.args.bytes[0] === 3));
    button("Disconnect").click(); await wait(() => text().includes("Disconnected"));
    await unlock(); await wait(() => lateUnlock);
    input("ssh-host", "changed.test"); await wait(() => !document.getElementById("ssh-username"));
    lateUnlock(); await wait(() => calls.some(call => call.command === "cancel_ssh_authentication" && call.args.selectionId === "app-selection-4"));
    check(!document.querySelector('input[name="ssh-identity"]'), "Late unlock restored a selection");
    await inspect("fixture.test"); await unlock(); await choose(); button("Open terminal").click();
    await wait(() => lateStart); input("ssh-host", "changed.test");
    await wait(() => !document.getElementById("ssh-username")); lateStart();
    await wait(() => calls.some(call => call.command === "close_ssh_session" && call.args.sessionId === "late-app-session"));
    check(!document.querySelector(".ssh-terminal"), "Late session restored the terminal");
    check(count("list_ssh_identities") === 0, "Application path accessed an agent");
    check(!JSON.stringify(Object.entries(localStorage)).includes(passphrase), "Passphrase persisted");
    await fetch("/__instance_fixture/result", {method:"POST",body:JSON.stringify({ok:true,checks:["application SSH consent/unlock/clear secret", "changed-host refusal", "explicit identity/authentication/terminal", "terminal Ctrl-C/disconnect", "stale unlock/start cleanup", "no agent or browser secret storage"]})});
  })().catch(async error => fetch("/__instance_fixture/result", {method:"POST",body:JSON.stringify({ok:false,error:`${error.message || error}\n${error.stack || ""}\nCommands: ${calls.map(call => call.command).join(",")}`})}));
})();
