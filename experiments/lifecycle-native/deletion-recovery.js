// Second owned app process: real native/server saved reads with writes/provider disabled.
(() => {
  const wait=async(predicate,stage)=>{const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error('Native snapshot recovery wait expired: '+stage+'; alerts='+[...document.querySelectorAll('[role="alert"]')].map(node=>node.textContent).join(' | '));await new Promise(resolve=>setTimeout(resolve,20));}};
  const text=()=>document.body.textContent;
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  function input(node,value){Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'),'mount');const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote mount');check(document.getElementById('remote-token').value==='','Previous token was restored');
    input(document.getElementById('remote-endpoint'),window.__snapshotFixture.endpoint);input(document.getElementById('remote-token'),window.__snapshotFixture.token);input(document.getElementById('remote-ca'),window.__snapshotFixture.ca);
    await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();
    await click('Load remote workspace');await wait(()=>document.querySelector('.deletion-recovery'),'recovery panel');
    check(!document.querySelector('.remote-deletion'),'Previous deletion form/intent was restored');
    const panel=document.querySelector('.deletion-recovery');check(panel.querySelector('input').value==='','Previous operation ID was restored implicitly');
    await click('Find deletion operations for VM 202');await click('Inspect deletion operation '+window.__snapshotFixture.operationId);
    await wait(()=>text().includes('Saved deletion status: succeeded'),'history recovery');
    check(text().includes('Saved deletion target: pve-test · VM 202'),'Saved request/target missing');
    check(text().includes('Current VM/storage state has not been checked; a reused VMID can belong to a different instance.'),'Historical completion was presented as current state');
    check(text().includes('Saved deletion verified: the VM and reviewed volumes were absent at completion.'),'Historical task/disk-only evidence missing');
    check(!panel.querySelector('input[type="checkbox"]')&&!text().includes('Confirm deletion of VM 202'),'Recovery exposed a submission control');
    input(panel.querySelector('input'),window.__snapshotFixture.operationId);await new Promise(resolve=>setTimeout(resolve,50));panel.querySelector('form').requestSubmit();
    await wait(()=>text().includes('Saved deletion status: succeeded')&&panel.getAttribute('aria-busy')==='false','saved-ID recovery');
    await click('Refresh saved deletion operation');await wait(()=>panel.getAttribute('aria-busy')==='false','saved operation refresh');
    await click('Disconnect');await wait(()=>!text().includes('Connected to'),'disconnect');
    await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:error.message}));
})();
