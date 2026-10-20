// Actual packaged creation UI -> native HTTPS -> production API/worker -> fixture.
import { writeFile } from "node:fs/promises";
import { join } from "node:path";
export async function exerciseCreate({check,delay,evaluate,click,input,wait,has,created,phrase,provider,requests,directory,ssh,directHttps=false}) {
  try {
  await wait(()=>evaluate("Boolean(document.getElementById('identity-passphrase'))"),"creation identity form");
  await evaluate("document.querySelector('details.app-identity').open=true");
  await input("identity-passphrase",phrase);await click("Unlock existing SSH identity");
  await wait(()=>has("Add this public key to the definition"),"same Android identity unlocked for creation");
  await click("Add this public key to the definition");
  check(await evaluate(`document.getElementById('create-public-key').value.trim()===${JSON.stringify(created.publicKey.trim())}`),"Creation did not retain the Android public key");
  check(!await evaluate("Boolean(document.querySelector('.remote-ssh-target'))"),"SSH target selected before creation");
  const creates=()=>requests.filter(request=>request.method==="POST"&&request.path==="/api/v1/provisioning/create").length;
  await click("Save definition and preview");await wait(()=>has("Review creation on"),"native creation plan review");
  check(creates()===0,"Preview submitted creation");
  check(await evaluate("[...document.querySelectorAll('button')].find(button=>button.textContent==='Create confirmed VM').disabled"),"Native creation skipped explicit confirmation");
  await evaluate("document.querySelector('.remote-create input[type=checkbox]').click()");
  await wait(()=>evaluate("![...document.querySelectorAll('button')].find(button=>button.textContent==='Create confirmed VM').disabled"),"confirmed creation enabled");
  await click("Create confirmed VM");await wait(()=>has("Refresh creation status"),"native creation acknowledgement");
  const deadline=Date.now()+30000;
  while(!await has("Guest initialized at 192.0.2.42")){
    check(Date.now()<deadline,`Native creation did not reach readiness; provider assertions: ${provider.errors.join(', ')}`);
    check(!await has("Creation: failed"),`Native creation failed; provider assertions: ${provider.errors.join(', ')}`);
    await wait(()=>evaluate("![...document.querySelectorAll('button')].find(button=>button.textContent==='Refresh creation status').disabled"),"creation refresh available");
    await click("Refresh creation status");await delay(250);
  }
  provider.assertComplete();check(creates()===1,"Native UI submitted creation more than once");
  check(!await evaluate("Boolean(document.querySelector('.remote-ssh-target'))"),"Readiness automatically selected an SSH target");
  await click("Prepare SSH connection");
  await wait(()=>evaluate("document.getElementById('ssh-host')?.value==='192.0.2.42'"),"ready target handed to native SSH screen");
  check(await has("debian@192.0.2.42"),"Creation-plan account not included in handoff review");
  check(!await evaluate("Boolean(document.getElementById('ssh-username'))"),"Handoff bypassed fresh host inspection");
  // The declared guest is simulated. The owned SSH peer uses the same authorized
  // key through the selected host transport, not this synthetic guest IP.
  const sshReport=await ssh();
  provider.assertComplete();check(creates()===1,"SSH/restart replayed creation");
  const directSsh = sshReport.target.includes("host alias");
  const report={httpsTransport:directHttps ? "direct emulator host alias; no ADB reverse" : "owned ADB reverse tunnel",platform:"android",service:"production API and sequential worker",provider:"generated local HTTPS protocol fixture; no real VM",publicKeyFingerprint:created.fingerprint,requests,providerRequests:provider.requests,ssh:sshReport,checks:["actual native definition/preview and explicit create confirmation", "one creation request and exactly one clone/seed/start/readiness execution", "requested resources and same Android public key in Cloud-Init", "guest readiness and explicit exact-address/account handoff", "native SSH/PTY using the same application key through explicit fixture endpoint override", "no creation replay during SSH/restart"],limitations:["Provider/guest readiness is simulated, not real infrastructure", `SSH endpoint explicitly overridden for the owned ${directSsh ? "host alias" : "ADB tunnel"}; synthetic guest IP is not contacted`, ...(directHttps && directSsh ? ["Emulator host networking, not deployed/physical-device networking"] : ["No complete direct-network proof"]), "No physical-device, full software-IME or release proof"]};
  await writeFile(join(directory,"create-report.json"),JSON.stringify(report,null,2)+"\n");
  console.log(`Android create-to-SSH fixture passed: ${report.checks.join(', ')}`);return report;
  } catch (error) {
    console.error("Owned creation diagnostics:",await evaluate("({identity:document.querySelector('details.app-identity')?.innerText?.slice(0,2000),alerts:[...document.querySelectorAll('.remote-create [role=alert]')].map(alert=>alert.innerText),status:[...document.querySelectorAll('.remote-create [role=status]')].map(status=>status.innerText)})"));
    console.error("Owned provider assertions:",provider.errors);
    throw error;
  }
}

