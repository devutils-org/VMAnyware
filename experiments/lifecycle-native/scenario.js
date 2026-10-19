// Drive the built React UI through genuine Tauri IPC; no invoke replacement.
(() => {
  const wait = async (predicate, stage = "initialization") => {
    const deadline = performance.now() + 20000;
    while (!predicate()) {
      if (performance.now() > deadline) throw Error("Native UI wait expired at " + stage + "; port=" + document.getElementById("port")?.value + "; body=" + document.body?.textContent?.slice(0, 4500));
      await new Promise(resolve => setTimeout(resolve, 20));
    }
  };
  const button = label => [...document.querySelectorAll("button")].find(node => node.textContent === label);
  const click = async label => { await wait(() => button(label) && !button(label).disabled, label); button(label).click(); };
  const text = () => document.body.textContent;
  const check = (value, message) => { if (!value) throw Error(message); };
  (async () => {
    await wait(() => document.getElementById("port") && window.__TAURI_INTERNALS__?.invoke);
    const input = document.getElementById("port");
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, String(window.__lifecycleFixturePort));
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
    await click("Check native bridge");
    await wait(() => text().includes("linux"));
    await click("Load registered instances");
    await click("Check lifecycle for VM 202");
    await click("Review start VM 202");
    check(button("Confirm start VM 202").disabled, "Consent was not required");
    await click("Cancel lifecycle confirmation");
    await click("Review start VM 202");
    document.querySelector('.lifecycle-actions input[type="checkbox"]').click();
    await click("Confirm start VM 202");
    await wait(() => text().includes("Lifecycle action: succeeded"));
    check(text().includes("power is running"), "Missing independent running power evidence");
    check(text().includes("Guest and SSH readiness are not verified"), "Readiness was inferred");
    await click("Acknowledge lifecycle result");
    await click("Check lifecycle for VM 202");
    await click("Review shutdown VM 202");
    document.querySelector('.lifecycle-actions input[type="checkbox"]').click();
    await click("Confirm shutdown VM 202");
    await wait(() => text().includes("Lifecycle action: succeeded") && text().includes("power is stopped"));
    await click("Acknowledge lifecycle result");
    await window.__TAURI_INTERNALS__.invoke("smoke_report", { ok: true, failure: null });
  })().catch(error => window.__TAURI_INTERNALS__.invoke("smoke_report", { ok: false, failure: String(error) }));
})();
