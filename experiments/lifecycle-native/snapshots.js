// Real WebView/IPC/HTTPS/server/provider journey; no invoke replacement.
(() => {
  const wait=async(predicate,stage)=>{const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error('Snapshot native wait expired: '+stage);await new Promise(resolve=>setTimeout(resolve,20));}};
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  const text=()=>document.body.textContent;
  const check=(condition,message)=>{if(!condition)throw Error(message);};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'),'mount');
    const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote mount');
    input('remote-endpoint',window.__snapshotFixture.endpoint);input('remote-token',window.__snapshotFixture.token);input('remote-ca',window.__snapshotFixture.ca);
    await new Promise(resolve=>setTimeout(resolve,50));
    document.querySelector('.remote-connection form').requestSubmit();
    await click('Load remote workspace');
    await click('List snapshots for VM 202');
    await wait(()=>text().includes('No snapshots reported for this VM'),'empty snapshot list');
    check(text().includes('Storage support has not been checked'),'Empty list claimed storage support');
    await click('List snapshots for VM 202');
    await wait(()=>text().includes('before-work'),'snapshot metadata');
    check(text().includes('kernel-lab · pve-test · VM 202'),'Retained target not shown');
    check(text().includes('Created: Unknown · RAM: Unknown')&&text().includes('may be incomplete'),'Unknown/incomplete metadata lost');
    check(text().includes('not independent backups'),'Backup distinction absent');
    check(text().includes('<b>literal description</b>')&&!document.querySelector('.remote-snapshots b'),'Description interpreted as HTML');
    check(!document.querySelector('.remote-snapshots').textContent.includes('current'),'Synthetic current was shown as a snapshot');
    await click('List snapshots for VM 202');
    await wait(()=>document.querySelector('.remote-snapshots [role="alert"]')?.textContent.includes('Cannot load snapshots'),'permission error');
    check(!text().includes('before-work')&&!text().includes('No snapshots reported'),'Error retained successful results');
    check(!text().includes('fixture-only-private-provider-detail'),'Raw provider error leaked');
    await click('Disconnect');
    await wait(()=>!text().includes('Connected to'),'disconnect');
    await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:error.message}));
})();
