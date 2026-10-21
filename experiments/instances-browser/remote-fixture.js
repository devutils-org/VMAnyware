// Isolated UI-only IPC fixture; no native connection, credentials or provider.
(() => {
  const token = "fixture-only-access-token-0123456789abcdef";
  const calls = [];
  const declared = Boolean(window.__declaredCreateFixture);
  let expectedPackages = [], expectedSnapshot = false, prepares = 0;
  const publicCa = "-----BEGIN CERTIFICATE-----\npublic-browser-fixture\n-----END CERTIFICATE-----";
  const appPublicKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
  const fixturePassphrase = "disposable identity phrase 42";
  let unlockReads = 0, vaultUnlocks = 0, vaultSaved = false;
  let complete;
  let reads = 0, releaseImages, rejectEnvironments;
  let submittedKey, createAttempts = 0, createReads = 0, releaseCreate;
  let historyReads = 0, recoveryReads = 0, releaseRecovery;
  let progressReads = 0, releaseProgress;
  let trustedSsh = false;
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = { invoke: async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "ssh_host_support") return true;
    if (command === "ssh_agent_support") return false;
    if (command === "cancel_ssh_host") return;
    if (command === "inspect_ssh_host") {
      check(args.host === "192.0.2.42" && args.port === 22, "Handoff changed the ready target");
      return {host:args.host,port:22,algorithm:"ssh-ed25519",fingerprint:"SHA256:handoff-fixture",status:trustedSsh ? "trusted" : "unknown",challengeId:trustedSsh ? null : "handoff-challenge",expiresInSeconds:trustedSsh ? 0 : 120};
    }
    if (command === "confirm_ssh_host") {
      check(args.challengeId === "handoff-challenge" && args.fingerprint === "SHA256:handoff-fixture", "Wrong handoff trust confirmation");
      trustedSsh = true;
      return {host:"192.0.2.42",port:22,fingerprint:args.fingerprint};
    }
    if (command === "identity_support" || command === "system_identity_support") return true;
    if (command === "remember_app_identity") {
      check(args.confirmed === true && args.passphrase === fixturePassphrase, "Vault save lacks exact secret/consent"); vaultSaved=true;
      return { publicKey: appPublicKey, fingerprint: "SHA256:fixture-identity" };
    }
    if (command === "unlock_system_app_identity") {
      check(Object.keys(args).length === 0, "Vault unlock received a renderer secret");
      if (++vaultUnlocks === 1 || !vaultSaved) throw "PRIVATE_KEYRING_ERROR";
      return { publicKey: appPublicKey, fingerprint: "SHA256:fixture-identity" };
    }
    if (command === "forget_app_identity") { check(args.confirmed === true && Object.keys(args).length === 1, "Vault forget lacks scoped consent"); vaultSaved=false; return; }
    if (command === "list_system_ssh_identity") {
      check(args.host === "192.0.2.42" && args.port === 22 && args.username === "debian" && !Object.hasOwn(args, "passphrase"), "Vault SSH changed target or sent secret");
      return {id:"vault-selection",host:args.host,port:args.port,username:args.username,identities:[{algorithm:"ssh-ed25519",fingerprint:"SHA256:fixture-identity"}],expiresInSeconds:120};
    }
    if (command === "cancel_ssh_authentication") return;
    if (command === "create_app_identity" || command === "unlock_app_identity") {
      check(args.passphrase === fixturePassphrase, "Wrong fixture passphrase");
      if (command === "unlock_app_identity" && ++unlockReads === 1) throw "Cannot unlock application SSH identity";
      return { publicKey: appPublicKey, fingerprint: "SHA256:fixture-identity" };
    }
    if (command === "disconnect_remote") return;
    if (command === "read_remote_images" || command === "read_remote_environments") {
      check(args.connectionId === 1, "Wrong connection scope");
      const image = command === "read_remote_images";
      if (image) reads++;
      if (reads === 1) return [];
      if (reads === 4) return new Promise((resolve, reject) => { if (image) releaseImages = resolve; else rejectEnvironments = reject; });
      if (image && reads === 3) throw "Catalog unavailable";
      return image ? [{image:"remote-debian",node:"fixture-node",templateVmid:9000,templateVersion:"v1",sourceDiskGiB:8,guestUser:"debian"}] : [{id:"a".repeat(32),definitionName:"remote-lab",providerReference:{node:"fixture-node",vmid:202},status:"registered"},{id:"9".repeat(32),definitionName:"retired-remote-lab",providerReference:{node:"fixture-node",vmid:203},status:"deleted"}];
    }
    if (command === "prepare_remote_create") {
      check(args.connectionId === 1, "Wrong creation connection");
      const definition = JSON.parse(args.yaml);
      check(JSON.stringify(definition.spec.provisioning.packages) === JSON.stringify(expectedPackages) && definition.spec.lifecycle.snapshotAfterProvisioning === expectedSnapshot, "Creation changed the declared packages/snapshot");
      prepares++;
      check(definition.spec.provisioning.sshAuthorizedKeys.length === 1, "Public key missing or duplicated");
      check(definition.spec.provisioning.sshAuthorizedKeys[0] === appPublicKey, "Application key not used");
      check(!args.yaml.includes(fixturePassphrase) && !args.yaml.includes("PRIVATE"), "Secret sent to control plane");
      submittedKey = args.requestKey;
      return {definitionName:"mobile-lab",definitionHash:"a".repeat(64),planHash:"b".repeat(64),creationEnabled:true,sshKeyCount:1,packages:expectedPackages,snapshotAfterProvisioning:declared&&prepares===1 ? !expectedSnapshot : expectedSnapshot,source:{templateVmid:9000,templateVersion:"fixture-v1"},target:{node:"fixture-node",vmid:202,storage:"fixture-storage",bridge:"vmbr0",guestUser:"debian",resources:{cpu:2,memoryMiB:2048,diskGiB:8}}};
    }
    if (command === "confirm_remote_create") {
      check(args.requestKey === submittedKey && args.confirmed === true && args.connectionId === 1, "Creation confirmation or retry key changed");
      if (++createAttempts === 1) throw "Fixture response lost; retain the request key.";
      return {id:"d".repeat(32),kind:"vm_provision",status:"queued",result:null};
    }
    if (command === "read_remote_create") {
      check(args.connectionId === 1 && args.requestKey === submittedKey, "Wrong operation read scope");
      if (++createReads <= 4) return {id:"d".repeat(32),kind:"vm_provision",status:createReads===1||createReads===4?"failed":"uncertain",result:null,failure:["packages_failed","packages_timed_out","snapshot_state_unconfirmed","PRIVATE_UNKNOWN_CODE"][createReads-1],message:"PRIVATE_GUEST_STDERR"};
      if (createReads === 6) return new Promise(resolve => { releaseCreate = resolve; });
      return {id:"d".repeat(32),kind:"vm_provision",status:"succeeded",result:{guestReady:true,guest:{ipv4:"192.0.2.42"}}};
    }
    if (command === "read_remote_provisioning_progress") {
      check(args.operationId === "d".repeat(32) && [1,3].includes(args.connectionId) && Object.keys(args).length === 2, "Progress supplied mutation input or wrong scope");
      progressReads++;
      const kinds = ["full_clone", "configure_resources", "configure_cloud_init", "start_vm", "wait_for_guest_readiness", "install_declared_packages", "snapshot_after_provisioning"];
      const value = {operationId:args.operationId,instanceId:"e".repeat(32),sourceInstanceId:null,definitionHash:"a".repeat(64),planHash:"b".repeat(64),node:args.connectionId===1?"fixture-node":"recovered-node",vmid:202,status:"uncertain",privateJournal:"PRIVATE_PROVIDER_OUTPUT",stages:kinds.map((kind,position)=>({position,kind,status:position===5?"uncertain":position<5?"succeeded":"queued",submissionStarted:position<=5}))};
      if(progressReads===2)value.planHash="f".repeat(64);
      if(progressReads===3)throw "PRIVATE_ENDPOINT_ERROR";
      if(progressReads===4)return new Promise(resolve=>{releaseProgress=()=>resolve({...value,node:"STALE_PROGRESS"});});
      return value;
    }
    if (command === "read_remote_creations") {
      check(args.connectionId === 3, "History read used the wrong connection");
      return ++historyReads === 1 ? [] : [{id:"d".repeat(32),kind:"vm_provision",status:"succeeded",instanceId:"e".repeat(32),createdAt:"2026-10-18T10:00:00Z"}];
    }
    if (command === "recover_remote_creation") {
      check(args.connectionId === 3 && args.operationId === "d".repeat(32), "Recovery used the wrong server or operation");
      if (++recoveryReads === 1) return {operation:{id:"d".repeat(32),status:"uncertain",result:null,failure:"packages_unconfirmed",message:"PRIVATE_REPOSITORY"},environment:{definitionName:"recovered-lab",providerReference:{node:"recovered-node",vmid:202}}};
      if (recoveryReads === 3) return new Promise(resolve => { releaseRecovery = resolve; });
      return {operation:{id:"d".repeat(32),status:"succeeded",result:{guestReady:true,guest:{ipv4:"192.0.2.42"}}},environment:{definitionName:"recovered-lab",providerReference:{node:"recovered-node",vmid:202}}};
    }
    if (command === "connect_remote") {
      if (args.token !== token || args.endpoint !== "https://fixture.example.test") throw "Unexpected fixture inputs";
      return new Promise(resolve => { complete = resolve; });
    }
    throw `Unexpected fixture command: ${command}`;
  } };
  const text = () => document.body?.textContent ?? "";
  const check = (ok, message) => { if (!ok) throw new Error(message); };
  async function wait(predicate) {
    const end = Date.now() + 5000;
    while (!predicate()) { if (Date.now() > end) throw new Error(`UI wait timed out (progress reads ${progressReads}): ${text().slice(-400)}`); await new Promise(resolve => setTimeout(resolve, 10)); }
  }
  function mode(value) { const element = document.getElementById("connection-mode"); element.value = value; element.dispatchEvent(new Event("change", { bubbles: true })); }
  function input(id, value) { const element = document.getElementById(id); Object.getOwnPropertyDescriptor(element instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype, "value").set.call(element, value); element.dispatchEvent(new Event("input", { bubbles: true })); }
  const button = label => [...document.querySelectorAll("button")].find(element => element.textContent === label);
  (async () => {
    await wait(() => document.getElementById("connection-mode"));
    mode("remote"); await wait(() => document.getElementById("remote-token"));
    check(!document.getElementById("port"), "Local port visible in remote mode");
    check(!button("Load registered instances"), "Local VM controls visible in remote mode");
    input("remote-endpoint", "https://fixture.example.test"); input("remote-token", token);
    input("remote-ca", publicCa);
    await wait(() => document.getElementById("remote-token").value === token && document.getElementById("remote-ca").value === publicCa);
    document.querySelector(".remote-connection form").requestSubmit();
    await wait(() => complete);
    check(document.getElementById("remote-token").value === "", "Token remained in the field during connection");
    check(calls.find(call=>call.command==="connect_remote").args.caPem===publicCa,"Explicit connection CA was not forwarded");
    check(!JSON.stringify(Object.entries(localStorage)).includes(token), "Token persisted in localStorage");
    complete({ connectionId: 1, endpoint: "https://fixture.example.test/" });
    await wait(() => text().includes("Connected to https://fixture.example.test/"));
    check(document.getElementById("remote-endpoint").disabled, "Connected address remained editable");
    check(document.getElementById("remote-ca").disabled, "Connected CA remained editable");
    await wait(() => document.getElementById("identity-passphrase"));
    document.querySelector("details.app-identity").open = true;
    input("identity-passphrase", fixturePassphrase); input("identity-confirmation", "different phrase for fixture");
    await new Promise(resolve => setTimeout(resolve, 20));
    check(button("Create encrypted SSH identity").disabled, "Mismatched passphrases allowed creation");
    input("identity-confirmation", fixturePassphrase);
    await wait(() => !button("Create encrypted SSH identity").disabled);
    button("Create encrypted SSH identity").click();
    await wait(() => button("Add this public key to the definition"));
    check(document.getElementById("identity-passphrase").value === "" && document.getElementById("identity-confirmation").value === "", "Passphrase fields retained secrets");
    await wait(()=>document.getElementById("remember-identity-consent"));
    check(!document.getElementById("remember-identity-consent").checked && button("Remember SSH unlock secret").disabled && button("Forget saved SSH unlock secret").disabled, "Vault save/forget consent enabled by default");
    check(!calls.some(call=>["remember_app_identity","unlock_system_app_identity","forget_app_identity"].includes(call.command)), "Vault accessed implicitly");
    input("identity-passphrase",fixturePassphrase);document.getElementById("remember-identity-consent").click();
    await wait(()=>!button("Remember SSH unlock secret").disabled);button("Remember SSH unlock secret").click();
    await wait(()=>text().includes("Unlock secret saved in the system key store"));
    check(document.getElementById("identity-passphrase").value===""&&!document.getElementById("remember-identity-consent").checked,"Vault save retained secret or consent");
    button("Unlock using system key store").click();await wait(()=>text().includes("System key store unavailable or unable"));
    check(!text().includes("PRIVATE_")&&!document.getElementById("identity-public-key"),"Failed vault unlock retained identity or private error");
    button("Unlock using system key store").click();await wait(()=>text().includes("Identity unlocked using the system key store"));
    document.getElementById("forget-identity-consent").click();await wait(()=>!button("Forget saved SSH unlock secret").disabled);button("Forget saved SSH unlock secret").click();
    await wait(()=>text().includes("Saved unlock secret forgotten"));check(!document.getElementById("forget-identity-consent").checked,"Forget consent survived action");
    button("Unlock using system key store").click();await wait(()=>text().includes("System key store unavailable or unable"));
    input("identity-passphrase",fixturePassphrase);document.getElementById("remember-identity-consent").click();await wait(()=>!button("Remember SSH unlock secret").disabled);button("Remember SSH unlock secret").click();await wait(()=>button("Add this public key to the definition"));
    button("Add this public key to the definition").click();
    await wait(() => document.getElementById("create-public-key").value === appPublicKey);
    button("Add this public key to the definition").click();
    input("identity-passphrase", fixturePassphrase);
    await new Promise(resolve => setTimeout(resolve, 20));
    document.querySelector(".app-identity form").requestSubmit();
    await wait(() => text().includes("Cannot unlock application SSH identity"));
    check(!document.getElementById("identity-public-key"), "Failed unlock retained an available identity");
    input("identity-passphrase", fixturePassphrase);
    await new Promise(resolve => setTimeout(resolve, 20));
    document.querySelector(".app-identity form").requestSubmit();
    await wait(() => button("Add this public key to the definition"));
    check(!JSON.stringify(Object.entries(localStorage)).includes(fixturePassphrase), "Passphrase persisted in browser storage");
    await new Promise(resolve => setTimeout(resolve, 20));
    check(document.getElementById("create-packages").value === "" && !document.getElementById("create-final-snapshot").checked, "Provisioning features were enabled by default");
    if (declared) {
      expectedPackages = ["build-essential", "git=1:2.0-1"]; expectedSnapshot = true;
      input("create-packages", expectedPackages.join("\n")); document.getElementById("create-final-snapshot").click();
      await new Promise(resolve => setTimeout(resolve, 20)); document.querySelector(".create-definition").requestSubmit();
      await wait(() => text().includes("server plan changed the declared packages"));
      check(!button("Create confirmed VM") && createAttempts === 0, "Changed server declarations allowed confirmation");
    }
    document.querySelector(".create-definition").requestSubmit();
    await wait(() => button("Create confirmed VM"));
    check(document.getElementById("create-packages").matches(":disabled") && document.getElementById("create-final-snapshot").matches(":disabled"), "Reviewed features remained editable");
    if (declared) {
      check(text().includes("Declared packages: build-essential, git=1:2.0-1") && text().includes("Requested · disk-only"), "Declared review omitted requested packages/snapshot");
      document.getElementById("create-confirm").click(); const oldKey = submittedKey;
      button("Edit definition").click(); await wait(() => !button("Create confirmed VM"));
      expectedPackages = ["git=1:2.0-2", "g++"]; expectedSnapshot = false;
      input("create-packages", expectedPackages.join("\n")); document.getElementById("create-final-snapshot").click();
      await new Promise(resolve => setTimeout(resolve, 20)); document.querySelector(".create-definition").requestSubmit();
      await wait(() => button("Create confirmed VM"));
      check(!document.getElementById("create-confirm").checked && button("Create confirmed VM").disabled && submittedKey !== oldKey, "Edited declarations reused consent/key");
      check(text().includes("Declared packages: git=1:2.0-2, g++") && text().includes("Final provisioning snapshot: Not requested"), "Edited plan summary stale");
    }
    check(button("Create confirmed VM").disabled, "Creation did not require explicit confirmation");
    check(text().includes("fixture-storage") && text().includes("fixture-node") && text().includes("2048 MiB"), "Incomplete target review");
    document.getElementById("create-confirm").click();
    await wait(() => !button("Create confirmed VM").disabled); button("Create confirmed VM").click();
    await wait(() => button("Retry same confirmed request") && !button("Retry same confirmed request").disabled);
    check(text().includes(submittedKey), "Request key lost after submission error");
    button("Retry same confirmed request").click();
    await wait(() => text().includes("Creation: queued") && !button("Refresh creation status").disabled);
    check(!button("Prepare SSH connection") && !document.querySelector('.remote-ssh-target'), "Queued creation offered SSH handoff");
    check(progressReads === 0, "Progress loaded implicitly");
    button("Read recorded steps").click(); await wait(()=>text().includes("Install declared packages: uncertain"));
    check(text().includes("Create final disk-only snapshot: queued") && text().includes("A started marker does not confirm completion") && !text().includes("PRIVATE_"), "Missing ordered stages or private progress text exposed");
    button("Refresh recorded steps").click(); await wait(()=>text().includes("Recorded steps are unavailable"));
    check(!text().includes("Install declared packages: uncertain"), "Mismatched plan retained old progress");
    button("Read recorded steps").click(); await wait(()=>progressReads===3); await new Promise(resolve=>setTimeout(resolve,30)); await wait(()=>!button("Read recorded steps").disabled);
    check(!text().includes("PRIVATE_"), "Progress error leaked private details");
    button("Read recorded steps").click(); await wait(()=>releaseProgress);
    button("Refresh creation status").click();
    await wait(() => text().includes("Creation: failed") && !button("Refresh creation status").disabled);
    check(!button("Prepare SSH connection") && !document.querySelector('.remote-ssh-target'), "Failed creation offered SSH handoff");
    releaseProgress(); await new Promise(resolve=>setTimeout(resolve,30)); check(!text().includes("STALE_PROGRESS"), "Old progress survived operation status replacement");
    check(text().includes("Declared package installation failed"), "Package failure reason missing");
    for (const message of ["Package installation exceeded its deadline", "snapshot task completed, but its exact disk-only snapshot is not confirmed", "Provisioning failure details are unavailable"]) {
      button("Refresh creation status").click(); await wait(() => text().includes(message) && !button("Refresh creation status").disabled);
      check(!text().includes("PRIVATE_") && !button("Prepare SSH connection"), "Failure exposed guest text or enabled SSH");
    }
    button("Refresh creation status").click();
    await wait(() => text().includes("Guest initialized at 192.0.2.42"));
    check(text().includes("SSH access and host identity still need verification"), "Guest readiness was confused with SSH proof");
    check(!document.querySelector('.remote-ssh-target'), "Readiness automatically selected an SSH endpoint");
    button("Prepare SSH connection").click();
    await wait(() => document.getElementById("ssh-host")?.value === "192.0.2.42");
    check(document.getElementById("ssh-port").value === "22" && text().includes("debian@192.0.2.42"), "Ready target/user were not shown for review");
    check(!calls.some(call=>call.command==="inspect_ssh_host") && !document.getElementById("ssh-username"), "Handoff automatically inspected or enabled authentication");
    await wait(() => !button("Inspect host key").disabled); button("Inspect host key").click();
    await wait(() => button("Trust this host key"));
    check(button("Trust this host key").disabled && !document.getElementById("ssh-username"), "Unverified host allowed authentication");
    document.getElementById("ssh-trust-heading").closest("section").querySelector('input[type="checkbox"]').click();
    await wait(() => !button("Trust this host key").disabled);button("Trust this host key").click();
    await wait(() => text().includes("Saved host-key trust"));button("Inspect host key").click();
    await wait(() => document.getElementById("ssh-username")?.value === "debian");
    await wait(()=>[...document.getElementById("ssh-source").options].some(option=>option.value==="system"));
    const sourceSelect=document.getElementById("ssh-source");sourceSelect.value="system";sourceSelect.dispatchEvent(new Event("change",{bubbles:true}));
    await wait(()=>!document.getElementById("ssh-key-passphrase"));
    check(button("Unlock connection identity").disabled && !calls.some(call=>call.command==="list_system_ssh_identity"),"System source reused consent or read a key implicitly");
    document.getElementById("ssh-source").closest("form").querySelector('input[type="checkbox"]').click();
    await wait(()=>!button("Unlock connection identity").disabled);button("Unlock connection identity").click();
    await wait(()=>document.querySelector('input[name="ssh-identity"]'));check(!calls.some(call=>call.command==="start_ssh_session"),"Vault selection opened SSH implicitly");
    const inspections = calls.filter(call=>call.command==="inspect_ssh_host").length;
    button("Prepare SSH connection").click();
    await wait(() => !document.getElementById("ssh-username"));
    check(calls.filter(call=>call.command==="inspect_ssh_host").length===inspections, "Reselecting target inspected automatically");

    button("Refresh creation status").click(); await wait(() => releaseCreate);
    button("Load remote workspace").click();
    await wait(() => text().includes("No templates configured") && text().includes("No environments registered"));
    button("Load remote workspace").click();
    await wait(() => text().includes("remote-debian") && text().includes("remote-lab"));
    check(text().includes("retired-remote-lab") && text().includes("deleted · definition and history retained"), "Remote deleted identity missing");
    check(!button("Prepare snapshot for VM 203"), "Deleted remote identity exposed snapshot controls");
    check(text().includes("8 GiB") && text().includes("Power state not observed."), "Remote details or registration qualification missing");
    button("Load remote workspace").click();
    await wait(() => text().includes("Catalog unavailable") && button("Load remote workspace"));
    check(text().includes("remote-lab") && !text().includes("remote-debian"), "Partial failure lost successful results or retained failed results");
    button("Load remote workspace").click(); await wait(() => releaseImages && rejectEnvironments);
    button("Disconnect").click(); await wait(() => !text().includes("Connected to https://fixture.example.test/"));
    check(!document.querySelector(".remote-ssh-target"), "Disconnect retained an SSH target");
    releaseImages([{image:"STALE_REMOTE_IMAGE"}]); rejectEnvironments("STALE_REMOTE_FAILURE");
    releaseCreate({id:"d".repeat(32),status:"succeeded",result:{guestReady:true,guest:{ipv4:"STALE_REMOTE_CREATE"}}});
    await new Promise(resolve => setTimeout(resolve, 50));
    check(!text().includes("STALE_REMOTE"), "Disconnected workspace response leaked");
    complete = null;
    input("remote-token", token); document.querySelector(".remote-connection form").requestSubmit();
    await wait(() => complete); const late = complete;
    mode("local"); await wait(() => document.getElementById("port"));
    late({ connectionId: 2, endpoint: "https://fixture.example.test/" });
    await new Promise(resolve => setTimeout(resolve, 50));
    check(!text().includes("Connected to https://fixture.example.test/"), "Late remote result leaked into local mode");
    mode("remote"); await wait(() => document.getElementById("remote-token"));
    check(document.getElementById("remote-token").value === "", "Token restored after remount");
    check(!text().includes("Connected to https://fixture.example.test/"), "Old connection restored after remount");
    complete = null;
    input("remote-endpoint", "https://fixture.example.test"); input("remote-token", token);
    await new Promise(resolve => setTimeout(resolve, 20));
    document.querySelector(".remote-connection form").requestSubmit(); await wait(() => complete);
    complete({connectionId:3,endpoint:"https://fixture.example.test/"});
    await wait(() => button("Find recent creations"));
    check(!text().includes("Creation: succeeded"), "Old submitted form was restored implicitly");
    button("Find recent creations").click(); await wait(() => text().includes("No creations in this server's recent operation history"));
    button("Find recent creations").click(); await wait(() => button(`Inspect ${"d".repeat(32)}`));
    button(`Inspect ${"d".repeat(32)}`).click();
    await wait(() => text().includes("Recovered creation: uncertain"));
    check(text().includes("Package installation is not confirmed") && !button("Prepare SSH connection") && !text().includes("PRIVATE_"), "Saved creation failure missing/leaked or offered SSH");
    button("Read recorded steps").click(); await wait(()=>text().includes("Recorded steps: uncertain · recovered-node"));
    check(!button("Prepare SSH connection"), "Recovered stages granted SSH");
    button("Refresh recovered creation").click();
    await wait(() => text().includes("Recovered creation: succeeded"));
    check(text().includes("recovered-lab") && text().includes("recovered-node") && text().includes("VM 202"), "Recovered target association missing");
    check(text().includes("This is saved evidence; current power and SSH access still need verification"), "Historical proof was presented as current state");
    button("Prepare SSH connection").click();
    await wait(() => document.getElementById("ssh-host")?.value === "192.0.2.42");
    await wait(() => !button("Inspect host key").disabled);button("Inspect host key").click();
    await wait(() => document.getElementById("ssh-username"));
    check(document.getElementById("ssh-username").value === "", "Recovery guessed or restored a guest username");

    check(calls.filter(call => call.command === "confirm_remote_create").length === 2, "Recovery submitted another creation");
    button("Refresh recovered creation").click(); await wait(() => releaseRecovery);
    mode("local"); await wait(() => document.getElementById("port"));
    releaseRecovery({operation:{id:"d".repeat(32),status:"succeeded",result:{guestReady:true,guest:{ipv4:"STALE_RECOVERY_IP"}}},environment:{definitionName:"STALE_RECOVERY_TARGET",providerReference:{node:"stale",vmid:999}}});
    await new Promise(resolve => setTimeout(resolve, 50));
    check(!text().includes("STALE_RECOVERY"), "Recovery result leaked across connection modes");
    check(calls.filter(call => call.command === "connect_remote").length === 3, "Unexpected connection replay");
    return { ok: true, checks: [...(declared ? ["declared packages and snapshot review", "changed server declarations refused", "edit invalidates consent and request key"] : []), "remote form", "local controls isolated", "token cleared/no storage", "connect/disconnect", "remote lists/empty states", "partial failure", "create confirmation/same-key retry/status", "safe package/snapshot failure and saved-failure messages", "read-only creation recovery after remount", "explicit ready-target SSH handoff with host-trust gate and account prefill", "reselection/disconnect clears SSH state", "application identity create/unlock/public-key flow", "system vault explicit save/unlock/forget, secret clearing and SSH consent", "stale workspace/create/recovery/connect responses suppressed"] };
  })().catch(error => ({ ok: false, error: error.message })).then(report => fetch("/__instance_fixture/result", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(report) }));
})();
