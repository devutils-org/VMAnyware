// Actual WebView -> retained native commands -> authenticated server/worker/provider.
// The Python runner owns every endpoint and injects one lost submission response.
(() => {
  const wait=async(predicate,stage)=>{const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error('Native restore wait expired: '+stage+'; alerts='+[...document.querySelectorAll('[role="alert"]')].map(node=>node.textContent).join(' | '));await new Promise(resolve=>setTimeout(resolve,20));}};
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  const text=()=>document.body.textContent;
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  const consents=()=>[...document.querySelectorAll('.remote-snapshot-restore input[type="checkbox"]')];
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function review(){await click('Review snapshot restoration');await wait(()=>button('Cancel restore review')&&!button('Cancel restore review').disabled,'restore review');}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'),'mount');const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote mount');
    input('remote-endpoint',window.__snapshotFixture.endpoint);input('remote-token',window.__snapshotFixture.token);input('remote-ca',window.__snapshotFixture.ca);
    await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();
    await click('Load remote workspace');await click('List snapshots for VM 202');await click('Review restore native-before-work for VM 202');
    await review();check(text().includes('VM is already stopped: failed'),'Running fixture target was not refused');
    check(button('Confirm destructive restore').disabled&&consents().every(node=>!node.checked&&node.disabled),'Running target enabled consent');
    await click('Cancel restore review');await review();
    check(text().includes('VM is already stopped: passed'),'Stopped target did not pass revalidation');
    check(consents().length===2&&consents().every(node=>!node.checked),'Review restored consent');
    await click('Cancel restore review');await review();
    check(button('Confirm destructive restore').disabled&&consents().every(node=>!node.checked),'Consent survived cancellation');
    check(text().includes('kernel-lab')&&text().includes('pve-test')&&text().includes('VM 202')&&text().includes('native-before-work'),'VM/snapshot review missing');
    check(text().includes('discards current VM data')&&text().includes('not independent backups'),'Restore consequences missing');
    check(button('Prepare snapshot for VM 202').disabled,'Review did not exclude snapshot creation');
    const key=document.querySelector('.remote-snapshot-restore .operation-id').textContent;
    consents()[0].click();await new Promise(resolve=>setTimeout(resolve,30));check(button('Confirm destructive restore').disabled,'Target consent alone allowed restore');
    consents()[0].click();consents()[1].click();await new Promise(resolve=>setTimeout(resolve,30));check(button('Confirm destructive restore').disabled,'Data-loss consent alone allowed restore');
    consents()[0].click();await click('Confirm destructive restore');
    await wait(()=>text().includes('Submission outcome unknown')&&button('Retry same restore request')&&!button('Retry same restore request').disabled,'lost response');
    check(!button('Cancel restore review')&&consents().every(node=>node.disabled),'Unknown submission could be replaced');
    check(document.querySelector('.remote-snapshot-restore .operation-id').textContent===key,'Unknown reply changed retained key');
    await click('Retry same restore request');await wait(()=>text().includes('Restore operation:'),'retained operation');
    const end=Date.now()+15000;
    while(!text().includes('Restore operation: succeeded')){
      check(Date.now()<end,'Restore operation did not reach provider-verified completion');
      await click('Refresh restore operation');await wait(()=>document.querySelector('.remote-snapshot-restore').getAttribute('aria-busy')==='false','operation read');
      await new Promise(resolve=>setTimeout(resolve,100));
    }
    check(text().includes('Provider rollback verified: native-before-work')&&text().includes('VM observed stopped'),'Task/snapshot/power evidence missing');
    check(text().includes('Recovered guest contents and application consistency remain unverified'),'Provider evidence claims guest recovery');
    check(!button('Retry same restore request'),'Known operation offered another submission');
    await click('Refresh restore operation');await wait(()=>document.querySelector('.remote-snapshot-restore').getAttribute('aria-busy')==='false','independent saved status');
    await click('List snapshots for VM 202');await wait(()=>document.querySelector('.remote-snapshots strong')?.textContent==='native-before-work','post-restore list');
    await click('Acknowledge restore result');await wait(()=>!text().includes('Reviewed restore:'),'acknowledge');
    await click('Disconnect');await wait(()=>!text().includes('Connected to'),'disconnect');
    await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:error.message}));
})();
