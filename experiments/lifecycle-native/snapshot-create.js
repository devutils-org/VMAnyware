// Real WebView -> production retained commands -> HTTPS/server/provider. No IPC mocks.
(() => {
  const wait=async(predicate,stage)=>{const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error('Native snapshot creation wait expired: '+stage+'; alerts='+[...document.querySelectorAll('[role="alert"]')].map(node=>node.textContent).join(' | '));await new Promise(resolve=>setTimeout(resolve,20));}};
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  const text=()=>document.body.textContent;
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'),'mount');const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote mount');
    input('remote-endpoint',window.__snapshotFixture.endpoint);input('remote-token',window.__snapshotFixture.token);input('remote-ca',window.__snapshotFixture.ca);
    await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();
    await click('Load remote workspace');await click('Prepare snapshot for VM 202');
    await wait(()=>document.getElementById('snapshot-name'),'snapshot form');input('snapshot-name','native-before-work');input('snapshot-description','Disk & café + work\nOnly'.replace('\\n','\n'));
    await new Promise(resolve=>setTimeout(resolve,50));await click('Review snapshot creation');
    await wait(()=>button('Cancel snapshot review')&&!button('Cancel snapshot review').disabled,'review');
    check(button('Confirm snapshot creation').disabled,'Snapshot consent was not required');
    check(text().includes('kernel-lab')&&text().includes('pve-test')&&text().includes('VM 202'),'Reviewed target missing');
    await click('Cancel snapshot review');await click('Review snapshot creation');
    await wait(()=>button('Cancel snapshot review')&&!button('Cancel snapshot review').disabled,'second review');
    check(button('Confirm snapshot creation').disabled,'Consent survived cancellation');
    check(document.getElementById('snapshot-name').disabled,'Review did not lock the input');
    document.querySelector('.remote-snapshot-create input[type="checkbox"]').click();await click('Confirm snapshot creation');
    await wait(()=>text().includes('Snapshot operation:'),'operation identity');
    const end=Date.now()+15000;
    while(!text().includes('Snapshot operation: succeeded')){
      check(Date.now()<end,'Snapshot operation failed to reach verified success');await click('Refresh snapshot operation');
      await wait(()=>document.querySelector('.remote-snapshot-create').getAttribute('aria-busy')==='false','operation read');
      await new Promise(resolve=>setTimeout(resolve,100));
    }
    check(text().includes('Snapshot creation verified: native-before-work'),'Task-only completion was shown');
    check(!button('Retry same snapshot request'),'Known snapshot exposed another POST');
    await click('List snapshots for VM 202');await wait(()=>document.querySelector('.remote-snapshots strong')?.textContent==='native-before-work','independent list');
    check(document.querySelector('.remote-snapshots').textContent.includes('Disk & café + work'),'Snapshot description changed');
    await click('Acknowledge snapshot result');await wait(()=>!text().includes('Reviewed snapshot:'),'acknowledgement');
    await click('Disconnect');await wait(()=>!text().includes('Connected to'),'disconnect');
    await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:error.message}));
})();
