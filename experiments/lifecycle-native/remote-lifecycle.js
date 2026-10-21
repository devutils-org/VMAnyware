// Actual WebView/native HTTPS/production worker; generated provider only, no IPC mock.
(() => {
  const text=()=>document.body.textContent;
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  async function wait(predicate,label){const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error(`Remote lifecycle wait expired: ${label}; ${[...document.querySelectorAll('[role="alert"]')].map(n=>n.textContent).join(' | ')}`);await new Promise(resolve=>setTimeout(resolve,20));}}
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function settled(){await wait(()=>document.querySelector('.remote-lifecycle')?.getAttribute('aria-busy')==='false','action settled');}
  const powerPanel=()=>document.querySelector('.remote-instance-status');
  async function readPower(expected){
    await click('Read power for VM 202');
    await wait(()=>powerPanel()?.textContent.includes(`Observed power: ${expected}`)&&powerPanel().getAttribute('aria-busy')==='false','explicit power read');
    check(!!powerPanel().querySelector('time')?.dateTime,'Read completion time absent');
    check(powerPanel().textContent.includes('may already be stale')&&powerPanel().textContent.includes('Power does not verify ownership, guest readiness'),'Power read implied ownership/readiness');
    check(![...powerPanel().querySelectorAll('button')].some(node=>node.textContent.includes('SSH')),'Power read offered implicit SSH');
  }
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'),'mount');const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote form');
    input('remote-endpoint',window.__snapshotFixture.endpoint);input('remote-token',window.__snapshotFixture.token);input('remote-ca',window.__snapshotFixture.ca);
    await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();
    await click('Load remote workspace');
    await readPower('stopped');
    for(const action of ['start','shutdown']){
      await click(`Prepare ${action} for VM 202`);await click(`Review remote ${action}`);await settled();
      check(button(`Confirm remote ${action}`).disabled,'Consent not required');
      check(!powerPanel().querySelector('time')&&powerPanel().textContent.includes('Power state not observed.')&&button('Read power for VM 202').disabled,'Action review retained old power evidence');
      check(text().includes('kernel-lab')&&text().includes('pve-test')&&text().includes('VM 202'),'Reviewed target absent');
      check(text().includes(`Observed power: ${action==='start'?'stopped':'running'}`),'Preflight power differs');
      await click('Cancel lifecycle review');await click(`Review remote ${action}`);await settled();
      check(!document.getElementById('remote-lifecycle-confirm').checked,'Consent survived cancellation');
      check(button(`Prepare ${action==='start'?'shutdown':'start'} for VM 202`).disabled&&button('Prepare deletion for VM 202').disabled,'Competing action not locked');
      document.getElementById('remote-lifecycle-confirm').click();await click(`Confirm remote ${action}`);
      await wait(()=>text().includes('No verified lifecycle operation ID received'),'lost accepted reply');
      check(!button('Cancel lifecycle review'),'Unknown submitted action clearable');
      await click('Retry same lifecycle request');await wait(()=>text().includes('Lifecycle operation:'),'same-key accepted');await settled();
      check(!button('Retry same lifecycle request'),'Known ID still offered POST');
      // Exercise an explicit GET even if the worker already finished before retry.
      await click('Refresh lifecycle operation');await settled();
      const end=Date.now()+15000;
      while(!text().includes('Lifecycle operation: succeeded')){
        check(Date.now()<end,'No verified lifecycle success');await click('Refresh lifecycle operation');await settled();await new Promise(resolve=>setTimeout(resolve,100));
      }
      check(text().includes(`power state verified: ${action==='start'?'running':'stopped'}`),'Task/power evidence absent');
      check(text().includes('Guest readiness and SSH access are not verified'),'Power success claimed guest readiness');
      if(action==='shutdown')check(text().includes('unsaved work may be lost'),'Graceful shutdown warning absent');
      await click('Acknowledge lifecycle result');await settled();
      await readPower(action==='start'?'running':'stopped');
    }
    await click('Disconnect');
    await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(async error=>{await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:String(error)});});
})();
