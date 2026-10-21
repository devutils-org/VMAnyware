// Actual WebView -> retained native commands -> authenticated server/worker/provider.
// The Python runner owns every endpoint and injects one lost submission response.
(() => {
  const wait=async(predicate,stage)=>{const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error('Native deletion wait expired: '+stage+'; alerts='+[...document.querySelectorAll('[role="alert"]')].map(node=>node.textContent).join(' | '));await new Promise(resolve=>setTimeout(resolve,20));}};
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  const text=()=>document.body.textContent;
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  const consents=()=>[...document.querySelectorAll('.remote-deletion input[type="checkbox"]')];
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function review(){await click('Review VM deletion');await wait(()=>button('Cancel deletion review')&&!button('Cancel deletion review').disabled,'deletion review');}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'),'mount');const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote mount');
    input('remote-endpoint',window.__snapshotFixture.endpoint);input('remote-token',window.__snapshotFixture.token);input('remote-ca',window.__snapshotFixture.ca);
    await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();
    await click('Load remote workspace');await click('Prepare deletion for VM 202');
    await review();check(text().includes('VM is already stopped: failed'),'Running VM allowed deletion');
    check(button('Confirm deletion of VM 202').disabled&&consents().every(node=>!node.checked&&node.disabled),'Running VM enabled consent');
    await click('Cancel deletion review');await review();
    check(text().includes('Server deletion: enabled · prerequisites: passed'),'Stopped target failed review');
    check(text().includes('fixture:vm-202-disk-0')&&text().includes('owned volume removal'),'Reviewed volume missing');
    await click('Cancel deletion review');await review();
    check(consents().length===2&&consents().every(node=>!node.checked),'Consent survived cancellation');
    consents()[0].click();await new Promise(resolve=>setTimeout(resolve,30));check(button('Confirm deletion of VM 202').disabled,'Target consent alone allowed deletion');
    consents()[0].click();consents()[1].click();await new Promise(resolve=>setTimeout(resolve,30));check(button('Confirm deletion of VM 202').disabled,'Data loss consent alone allowed deletion');
    consents()[0].click();await click('Confirm deletion of VM 202');
    await wait(()=>text().includes('No verified operation ID received.')&&button('Retry the same deletion request')&&!button('Retry the same deletion request').disabled,'lost HTTP reply');
    check(!button('Cancel deletion review')&&consents().every(node=>node.disabled),'Unknown submission could be replaced');
    await click('Retry the same deletion request');await wait(()=>text().includes('Deletion status:'),'retained operation');
    const end=Date.now()+15000;
    while(!text().includes('Deletion status: succeeded')){
      check(Date.now()<end,'Deletion never reached verified completion');await click('Refresh deletion status');
      await wait(()=>document.querySelector('.remote-deletion').getAttribute('aria-busy')==='false','status read');await new Promise(resolve=>setTimeout(resolve,100));
    }
    check(text().includes('Deletion verified: the VM and reviewed volumes are absent. Definition and history retained.'),'Verified evidence missing');
    check(!button('Retry the same deletion request'),'Known operation offered resubmission');
    await click('Acknowledge deletion result');await wait(()=>!text().includes('Reviewed deletion:'),'acknowledgement');
    check(!button('Prepare deletion for VM 202'),'Deleted row offered another deletion');
    await click('Disconnect');await wait(()=>!text().includes('Connected to'),'disconnect');
    await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:error.message}));
})();
