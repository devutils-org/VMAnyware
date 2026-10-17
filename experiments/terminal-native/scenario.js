// Real native IPC only. Generated public metadata comes from the owned fixture.
(() => {
  const target = window.__nativeSmoke;
  let step = 0;
  const invoke = (command, args = {}) => window.__TAURI_INTERNALS__.invoke(command, args);
  const button = text => [...document.querySelectorAll("button")].find(node => node.textContent === text);
  const check = (value, message) => { if (!value) throw new Error(message); };
  const wait = async (predicate, description) => {
    step++;
    const deadline = Date.now() + 6000;
    while (!predicate()) {
      if (Date.now() >= deadline) throw new Error(`Timed out: ${description}`);
      await new Promise(resolve => setTimeout(resolve, 10));
    }
  };
  const input = (id, value) => {
    const node = document.getElementById(id);
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(node, String(value));
    node.dispatchEvent(new Event("input", { bubbles: true }));
  };
  const text = () => [...(document.querySelector(".xterm-rows")?.children ?? [])].map(row => row.textContent).join("\n");
  const type = value => {
    const node = document.querySelector(".xterm-helper-textarea");
    node.value = value;
    node.dispatchEvent(new InputEvent("input", { inputType: "insertText", data: value, bubbles: true }));
  };
  window.addEventListener("DOMContentLoaded", async () => {
    try {
      await wait(() => button("Inspect host key") && !button("Inspect host key").disabled, "native support");
      input("ssh-host", target.host); input("ssh-port", target.port);
      button("Inspect host key").click();
      await wait(() => button("Trust this host key"), "unknown host key");
      check(document.querySelector("#ssh-trust-heading").parentElement.textContent.includes(target.fingerprint), "Fixture host fingerprint mismatch");
      document.querySelector('#ssh-trust-heading').parentElement.querySelector('input[type="checkbox"]').click();
      button("Trust this host key").click();
      await wait(() => document.body.textContent.includes("Saved host-key trust"), "persisted fixture trust");
      button("Inspect host key").click();
      await wait(() => document.getElementById("ssh-username"), "fresh trusted handshake");
      input("ssh-username", "fixture");
      document.getElementById("ssh-username").closest("form").querySelector('input[type="checkbox"]').click();
      for (let connection = 0; connection < 2; connection++) {
        button("List agent identities").click();
        await wait(() => document.querySelector('input[name="ssh-identity"]'), "real agent identity listing");
        check(button("Open terminal").disabled, "Identity was preselected");
        const identity = document.querySelector('input[name="ssh-identity"]');
        check(identity.value === target.identity, "Wrong generated identity");
        identity.click(); button("Open terminal").click();
        await wait(() => text().includes("fixture> "), "real shell prompt rendered by xterm");
        document.querySelector(".ssh-terminal").scrollIntoView();
        type("stty -echo; printf 'NATIVE_%s_OK\\n' UI\n");
        await wait(() => text().includes("NATIVE_UI_OK"), "shell stdout");
        const terminal = document.querySelector(".ssh-terminal");
        for (const destination of ["definitions-heading", "plan-heading", "connection-heading", "ssh-trust-heading"]) {
          document.querySelector(`nav a[href="#${destination}"]`).click();
          await new Promise(resolve => setTimeout(resolve, 0));
          check(document.activeElement.id === destination, "Workspace navigation did not focus its destination");
          check(document.querySelector(".ssh-terminal") === terminal, "Workspace navigation replaced the terminal");
        }
        type("printf 'AFTER_%s_OK\\n' NAVIGATION\n");
        await wait(() => text().includes("AFTER_NAVIGATION_OK"), "same shell remains usable after workspace navigation");
        terminal.style.height = "320px";
        await new Promise(resolve => setTimeout(resolve, 200));
        const rows = document.querySelector(".xterm-rows").children.length;
        check(rows > 0 && rows < 24, "Fixture height did not resize the renderer");
        type("stty size\n");
        await wait(() => new RegExp(`\\b${rows} [1-9][0-9]*\\b`).test(text()), "real stty rows match resized renderer");
        type("printf 'SLEEPING\\n'; sleep 5\n");
        await wait(() => text().includes("SLEEPING"), "foreground command");
        await new Promise(resolve => setTimeout(resolve, 100));
        const interruptedAt = Date.now();
        button("Send Ctrl-C").click();
        await new Promise(resolve => setTimeout(resolve, 100));
        type("printf 'INTERRUPTED:%s\\n' \"$?\"\n");
        await wait(() => text().includes("INTERRUPTED:130"), "foreground Ctrl-C/status 130");
        check(Date.now() - interruptedAt < 2000, "Ctrl-C exceeded two seconds");
        button("Disconnect").click();
        await wait(() => document.body.textContent.includes("Disconnected"), "explicit disconnect");
      }
      await invoke("smoke_rotate_host");
      button("Inspect host key").click();
      await wait(() => document.body.textContent.includes("This host key differs from the trusted key"), "changed host key rejected");
      check(!button("Trust this host key"), "Changed key offered trust overwrite");
      check(!button("List agent identities"), "Changed host offered agent access");
      await invoke("smoke_report", { ok: true, step });
    } catch (error) {
      console.error(String(error));
      document.body.append(document.createTextNode(`Native smoke failure: ${error}`));
      await invoke("smoke_report", { ok: false, step });
    }
  });
})();
