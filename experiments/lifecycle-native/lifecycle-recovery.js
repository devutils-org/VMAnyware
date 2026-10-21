// Second owned app: production native GET readers, restarted server, no provider/writes.
(() => {
  const text=()=>document.body.textContent;
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  async function wait(predicate,label){const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error(`Native lifecycle recovery wait expired: ${label}; ${[...document.querySelectorAll('[role="alert"]')].map(n=>n.textContent).join(' | ')}`);await new Promise(resolve=>setTimeout(resolve,20));}}
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(node,value){Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'),'mount');const selector=document.getElementById('connection-mode');selector.value='remote';selector.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote form');check(document.getElementById('remote-token').value==='','Token restored without entry');
    input(document.getElementById('remote-endpoint'),window.__snapshotFixture.endpoint);input(document.getElementById('remote-token'),window.__snapshotFixture.token);input(document.getElementById('remote-ca'),window.__snapshotFixture.ca);
    await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');
    await wait(()=>document.querySelector('.lifecycle-recovery'),'saved lifecycle panel');const panel=document.querySelector('.lifecycle-recovery');
    check(panel.querySelector('input').value===''&&!document.querySelector('.remote-lifecycle'),'Intent or operation ID restored');
    await click('Find lifecycle operations for VM 202');
    for(const kind of ['vm_start','vm_shutdown']){
      const id=window.__snapshotFixture.operationIds[kind];await click('Inspect lifecycle operation '+id);
      await wait(()=>panel.textContent.includes('Saved lifecycle status: succeeded')&&panel.getAttribute('aria-busy')==='false','saved detail');
      check(panel.textContent.includes(kind==='vm_start'?'Saved lifecycle action: start':'Saved lifecycle action: graceful shutdown'),'Saved action missing');
      check(panel.textContent.includes(`were verified: ${kind==='vm_start'?'running':'stopped'}`),'Recorded task/power evidence missing');
      check(panel.textContent.includes('Current VM existence, power, guest readiness and SSH host identity have not been checked'),'Historical power treated as current state');
    }
    check(!panel.querySelector('input[type="checkbox"]')&&![...panel.querySelectorAll('button')].some(node=>/confirm|retry|SSH/.test(node.textContent)),'History restored consent or live SSH action');
    input(panel.querySelector('input'),window.__snapshotFixture.operationIds.vm_start);await new Promise(resolve=>setTimeout(resolve,50));panel.querySelector('form').requestSubmit();
    await wait(()=>panel.textContent.includes('Saved lifecycle action: start')&&panel.getAttribute('aria-busy')==='false','copied-ID lookup');
    await click('Refresh saved lifecycle operation');await wait(()=>panel.getAttribute('aria-busy')==='false','saved refresh');
    await window.__checkRetainedDetail();
    await click('Disconnect');await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:String(error)}));
})();
