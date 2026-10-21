// Real React saved-record controls, simulated read-only IPC; no native/server/VM.
(() => {
  const calls=[],id='a'.repeat(32),other='f'.repeat(32),scope='b'.repeat(64),saved='e'.repeat(32),failed='d'.repeat(32);
  const environments=[{id,definitionName:'saved-deletion-lab',providerReference:{scopeId:scope,node:'saved-node',vmid:202},status:'registered'},{id:other,definitionName:'retired-lab',providerReference:{scopeId:scope,node:'retired-node',vmid:203},status:'deleted'}];
  let connection=0,histories=0,details=0,releaseDetail,rejectHistory;
  function record(operationId,status='succeeded') {
    return {target:{instanceId:id,providerScopeId:scope,definitionHash:'c'.repeat(64),node:'saved-node',vmid:202},configDigest:'d'.repeat(40),approvalHash:'9'.repeat(64),submissionStarted:status!=='failed',plan:{scopeHash:'8'.repeat(64),volumes:[{volume:'storage:vm-202-disk-0',references:['live:scsi0','snapshot/before-work:scsi0'],ownerVerified:true,effect:'delete_owned_volume'}],snapshots:['before-work'],preservedMedia:['local:iso/debian.iso'],preservedOwnedVolumes:['storage:vm-202-orphan']},operation:{id:operationId,kind:'vm_delete',instanceId:id,status,providerTaskKnown:status!=='failed',failure:status==='succeeded'?null:status==='failed'?'deletion_preconditions_failed':'deletion_state_unconfirmed',result:status==='succeeded'?{instanceId:id,providerScopeId:scope,node:'saved-node',vmid:202,approvalHash:'9'.repeat(64),taskStatus:'succeeded',vmAbsenceVerified:true,volumesAbsenceVerified:true,definitionRetained:true,associationRetired:true}:null}};
  }
  window.isTauri=true;
  window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});
    if(command.endsWith('_support'))return false;
    if(command==='connect_remote')return {connectionId:++connection,endpoint:'https://fixture.example.test/'};
    if(command==='disconnect_remote'||command==='cancel_ssh_host')return;
    if(command==='read_remote_images')return [];
    if(command==='read_remote_environments')return structuredClone(environments);
    if(command==='read_remote_deletion_history'){
      check(args.connectionId===connection,'History connection changed');
      if(args.instanceId===other)return {instanceId:other,operations:[]};
      check(args.instanceId===id,'History instance changed');histories++;
      if(histories===1)return {instanceId:id,operations:[]};
      if(histories===3)return {instanceId:other,operations:[]};
      if(histories===4)throw 'Cannot read saved deletion operations. Retry the read.';
      if(histories===5)return new Promise((resolve,reject)=>{rejectHistory=reject;});
      return {instanceId:id,operations:[{id:saved,status:histories===2?'uncertain':'succeeded',approvalHash:'9'.repeat(64),createdAt:'2026-10-21T10:00:00Z'}]};
    }
    if(command==='recover_remote_deletion'){
      check(args.connectionId===connection&&args.instanceId===id,'Saved detail scope changed');details++;
      if(details===1)return record(args.operationId,'uncertain');
      if(details===2){environments[0].status='deleted';return record(args.operationId);}
      if(details===3){const value=record(args.operationId);value.target.vmid=999;return value;}
      if(details===4){const value=record(args.operationId);value.operation.result.approvalHash='7'.repeat(64);return value;}
      if(details===5)return record(args.operationId,'failed');
      if(details===6){const value=record(args.operationId,'failed');return new Promise(resolve=>{releaseDetail=()=>resolve(value);});}
      return record(args.operationId);
    }
    throw Error(`Unexpected saved-deletion fixture command: ${command}`);
  }};
  const text=()=>document.body?.textContent||'';
  function check(ok,message){if(!ok)throw Error(message);}
  async function wait(predicate,stage='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,`Saved deletion wait expired: ${stage}; history=${histories}; detail=${details}; ${text().slice(-500)}`);await new Promise(resolve=>setTimeout(resolve,10));}}
  const panel=vmid=>document.querySelector(`.deletion-recovery[aria-label="Saved deletion operations for VM ${vmid}"]`);
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(node,value){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function connect(){await wait(()=>document.getElementById('remote-token'));input(document.getElementById('remote-endpoint'),'https://fixture.example.test');input(document.getElementById('remote-token'),'fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await wait(()=>panel(202)&&panel(203));}
  async function manual(operationId){input(panel(202).querySelector('input'),operationId);await new Promise(resolve=>setTimeout(resolve,20));await click('Read saved deletion operation');}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));await connect();
    check(histories===0&&details===0,'Recovery read ran automatically');
    check(!button('Prepare deletion for VM 203')&&!!button('Find deletion operations for VM 203'),'Retired row lost history or retained destructive controls');
    await click('Find deletion operations for VM 203');await wait(()=>panel(203).textContent.includes('No deletion operations'));
    await click('Find deletion operations for VM 202');await wait(()=>panel(202).textContent.includes('No deletion operations'));
    await click('Find deletion operations for VM 202');await wait(()=>button(`Inspect deletion operation ${saved}`));
    await click(`Inspect deletion operation ${saved}`);await wait(()=>panel(202).textContent.includes('Saved deletion status: uncertain'));
    check(panel(202).textContent.includes('Cleanup may be partial')&&panel(202).textContent.includes('never retries DELETE'),'Partial cleanup wording missing');
    check(panel(202).textContent.includes('storage:vm-202-disk-0')&&panel(202).textContent.includes('snapshot/before-work:scsi0')&&panel(202).textContent.includes('local:iso/debian.iso')&&panel(202).textContent.includes('storage:vm-202-orphan'),'Saved reviewed/preserved scope missing');
    check(panel(202).textContent.includes('No job purge or bulk unreferenced-disk deletion'),'Saved fixed effects missing');
    await click('Refresh saved deletion operation');await wait(()=>panel(202).textContent.includes('Saved deletion verified:'));
    check(panel(202).textContent.includes('Current VM/storage state has not been checked')&&panel(202).textContent.includes('reused VMID')&&panel(202).textContent.includes('Definition and history were retained'),'Historical/current distinction missing');
    await click('Load remote workspace');await wait(()=>!button('Prepare deletion for VM 202'));
    check(!!button('Find deletion operations for VM 202')&&panel(202).textContent.includes('Saved deletion verified:'),'Retirement removed saved recovery controls/state');
    await click('Refresh saved deletion operation');await wait(()=>panel(202).textContent.includes('different target or request'));
    check(!panel(202).textContent.includes('Saved deletion verified:'),'Wrong-target read retained old success');
    await manual(saved);await wait(()=>panel(202).textContent.includes('success lacks matching'));
    check(!panel(202).textContent.includes('Saved deletion verified:'),'Wrong full-input hash accepted');
    await manual(failed);await wait(()=>panel(202).textContent.includes('Saved deletion status: failed'));
    check(panel(202).textContent.includes('failed its deletion prerequisites'),'Recorded failed attempt missing');
    await click('Find deletion operations for VM 202');await wait(()=>panel(202).textContent.includes('history belongs to a different instance'));
    check(!panel(202).textContent.includes('No deletion operations'),'History scope error was shown as empty success');
    await click('Find deletion operations for VM 202');await wait(()=>panel(202).textContent.includes('Cannot read saved deletion operations'));
    const late='6'.repeat(32);await manual(late);await wait(()=>releaseDetail);
    input(panel(202).querySelector('input'),saved);releaseDetail();await new Promise(resolve=>setTimeout(resolve,50));
    check(!panel(202).textContent.includes('Saved deletion status:'),'Old detail survived ID editing');
    await click('Read saved deletion operation');await wait(()=>panel(202).textContent.includes('Saved deletion verified:'));
    await click('Find deletion operations for VM 202');await wait(()=>rejectHistory);
    await click('Disconnect');await connect();rejectHistory('STALE_DELETION_HISTORY_ERROR');await new Promise(resolve=>setTimeout(resolve,50));
    check(!text().includes('STALE_DELETION_HISTORY_ERROR')&&!panel(202).textContent.includes('Saved deletion verified:'),'Old recovery state/error survived reconnect');
    check(panel(202).querySelector('input').value===''&&!panel(202).querySelector('input[type="checkbox"]'),'Saved ID or consent was restored automatically');
    await click('Find deletion operations for VM 202');await wait(()=>button(`Inspect deletion operation ${saved}`));await manual(saved);await wait(()=>panel(202).textContent.includes('Saved deletion verified:'));
    check(!calls.some(call=>/^(prepare|confirm|clear)_remote_(deletion|snapshot)/.test(call.command)),'Read-only recovery invoked a mutation-intent command');
    await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['explicit empty/history reads','registered and retired rows','scoped row/manual-ID recovery','saved reviewed and preserved scope','historical/current distinction','partial cleanup retained','wrong target/hash refused','failed attempt visible','empty/error separation','stale edited-ID reply suppressed','disconnected history error suppressed','fresh-connection recovery without consent or mutation commands']})});
  })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
