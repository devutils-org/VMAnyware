// Actual WebView -> retained native commands -> authenticated server/worker/provider.
// The Python runner owns every endpoint and injects one lost submission response.
(() => {
  const wait=async(predicate,stage)=>{const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error('Native recreation wait expired: '+stage+'; alerts='+[...document.querySelectorAll('[role="alert"]')].map(node=>node.textContent).join(' | '));await new Promise(resolve=>setTimeout(resolve,20));}};
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  const text=()=>document.body.textContent;
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  const consents=()=>[...document.querySelectorAll('.remote-recreation input[type="checkbox"]')];
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function review(){await click('Review recreation');await wait(()=>button('Cancel recreation review')&&!button('Cancel recreation review').disabled,'review');}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'),'mount');const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote mount');input('remote-endpoint',window.__snapshotFixture.endpoint);input('remote-token',window.__snapshotFixture.token);input('remote-ca',window.__snapshotFixture.ca);
    await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await click('Prepare recreation from VM 202');
    input('recreation-vmid','202');await review();check(text().includes('Target VMID was observed free: passed'),'Target availability review missing');
    check(text().includes('Previous disk contents and snapshots are not restored')&&text().includes('revision 1')&&text().includes('mobile-lab'),'Retained source/fresh disk semantics missing');
    if(window.__snapshotFixture.declared)check(text().includes('Declared packages: build-essential, git=1:2.0-1')&&text().includes('Requested · disk-only'),'Declared native recreation summary missing');
    check(consents().length===1&&!consents()[0].checked&&button('Confirm recreation').disabled,'Consent preselected');
    consents()[0].click();await click('Cancel recreation review');await review();check(!consents()[0].checked&&button('Confirm recreation').disabled,'Consent survived cancellation');
    consents()[0].click();await click('Confirm recreation');await wait(()=>text().includes('No verified operation ID received.')&&button('Retry the same recreation request')&&!button('Retry the same recreation request').disabled,'lost response');
    check(!button('Cancel recreation review')&&consents()[0].disabled,'Unknown request could be replaced');
    await click('Retry the same recreation request');await wait(()=>text().includes('Recreation status:'),'retained operation');
    const end=Date.now()+20000;
    while(!text().includes('Recreation status: succeeded')){
      check(Date.now()<end,'Recreation never reached guest-ready completion');await click('Refresh recreation status');await wait(()=>document.querySelector('.remote-recreation').getAttribute('aria-busy')==='false','operation read');await new Promise(resolve=>setTimeout(resolve,100));
    }
    check(text().includes('New guest initialized at 192.0.2.42')&&!button('Retry the same recreation request'),'Missing new guest evidence or known operation resubmission');
    if(window.__snapshotFixture.declared){await click('Read recorded steps');await wait(()=>text().includes('Create final disk-only snapshot: succeeded'),'recorded native recreation stages');check(text().includes('Install declared packages: succeeded')&&text().includes('Saved stage evidence only'),'Native recreation progress incomplete');}
    await click('Prepare SSH for recreated VM');await wait(()=>text().includes('SSH target: debian@192.0.2.42'),'SSH trust handoff');check(text().includes('independently verify its host key'),'Host trust bypassed');
    await click('Acknowledge recreation result');await wait(()=>!text().includes('Reviewed recreation:'),'acknowledge');
    await click('Disconnect');await wait(()=>!text().includes('Connected to'),'disconnect');
    await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:error.message}));
})();
