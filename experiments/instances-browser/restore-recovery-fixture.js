// Actual React recovery controls with simulated read-only IPC, no native/server/VM.
(() => {
  const calls=[], id='a'.repeat(32), scope='b'.repeat(64), saved='e'.repeat(32);
  const environment={id,definitionName:'saved-lab',providerReference:{scopeId:scope,node:'saved-node',vmid:202},status:'registered'};
  let connection=0,histories=0,details=0,releaseDetail,rejectHistory;
  function record(operationId,name='before-work',status='succeeded') {
    return {target:{instanceId:id,providerScopeId:scope,definitionHash:'c'.repeat(64),node:'saved-node',vmid:202},request:{snapshotName:name},snapshotConfigHash:'9'.repeat(64),submissionStarted:status!=='failed',operation:{id:operationId,kind:'vm_snapshot_restore',instanceId:id,status,providerTaskKnown:status==='succeeded',failure:status==='succeeded'?null:status==='failed'?'restore_preconditions_failed':'provider_submission_unconfirmed',result:status==='succeeded'?{instanceId:id,providerScopeId:scope,node:'saved-node',vmid:202,taskStatus:'succeeded',snapshotName:name,snapshotConfigHash:'9'.repeat(64),observation:{node:'saved-node',vmid:202,status:'stopped',qmpStatus:null}}:null}};
  }
  window.isTauri=true;
  window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});
    if(command.endsWith('_support'))return false;
    if(command==='connect_remote')return {connectionId:++connection,endpoint:'https://fixture.example.test/'};
    if(command==='disconnect_remote' || command==='cancel_ssh_host')return;
    if(command==='read_remote_images')return [];
    if(command==='read_remote_environments')return [environment];
    if(command==='read_remote_restore_history'){
      check(args.connectionId===connection && args.instanceId===id,'History scope changed');histories++;
      if(histories===1)return {instanceId:id,operations:[]};
      if(histories===3)return {instanceId:'f'.repeat(32),operations:[]};
      if(histories===4)throw 'Cannot read saved operations. Retry the read.';
      if(histories===5)return new Promise((resolve,reject)=>{rejectHistory=reject;});
      return {instanceId:id,operations:[{id:saved,snapshotName:'before-work',status:'succeeded',createdAt:'2026-10-20T10:00:00Z'}]};
    }
    if(command==='recover_remote_restore'){
      check(args.connectionId===connection && args.instanceId===id,'Recovery scope changed');details++;
      if(details===2)return record(args.operationId,'before-work','uncertain');
      if(details===3)return {...record(args.operationId),target:{...record(args.operationId).target,vmid:999}};
      if(details===4){const result=record(args.operationId);result.operation.result.snapshotConfigHash='7'.repeat(64);return result;}
      if(details===5)return record(args.operationId,'failed-work','failed');
      if(details===6)return new Promise(resolve=>{releaseDetail=()=>resolve(record(args.operationId,'STALE_DETAIL'));});
      return record(args.operationId);
    }
    throw `Unexpected recovery command: ${command}`;
  }};
  function check(ok,message){if(!ok)throw Error(message);}
  const text=()=>document.body?.textContent || '';
  const panel=()=>document.querySelector('.restore-recovery');
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  async function wait(predicate,stage='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,`Restore recovery wait expired: ${stage}; history=${histories}; detail=${details}`);await new Promise(resolve=>setTimeout(resolve,10));}}
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(node,value){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function connect(){input(document.getElementById('remote-endpoint'),'https://fixture.example.test');input(document.getElementById('remote-token'),'fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await wait(()=>panel(),'recovery mount');}
  async function readId(value){input(panel().querySelector('input'),value);await new Promise(resolve=>setTimeout(resolve,20));panel().querySelector('form').requestSubmit();}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'));await connect();check(histories===0&&details===0,'Recovery read happened automatically');
    await click('Find restore operations for VM 202');await wait(()=>text().includes("No restore operations in this instance's recent history"));
    await click('Find restore operations for VM 202');await click('Inspect restore operation '+saved);
    await wait(()=>text().includes('Saved restore operation: succeeded'));
    check(text().includes('saved-lab')&&text().includes('Saved restore request: before-work · saved-node · VM 202'),'Saved target/request missing');
    check(text().includes('Current VM state, snapshot existence and recovered guest contents have not been checked'),'Saved evidence claimed current state');
    check(!panel().querySelector('input[type="checkbox"]')&&!panel().textContent.includes('Confirm'),'Recovery offered mutation consent');
    await click('Refresh saved restore operation');await wait(()=>text().includes('Saved restore operation: uncertain'));
    check(text().includes('no task identity was confirmed')&&!text().includes('The recorded provider task, selected snapshot fingerprint and stopped target matched'),'Unknown submission displayed success');
    await click('Find restore operations for VM 202');await wait(()=>text().includes('history belongs to a different instance'));
    check(!text().includes('Saved restore operation: uncertain'),'Invalid history retained old detail');
    await click('Find restore operations for VM 202');await wait(()=>text().includes('Cannot read saved operations'));
    check(!text().includes('No restore operations in this instance'),'Read failure became an empty success');
    await readId('f'.repeat(32));await wait(()=>text().includes('operation belongs to a different target'));
    check(!text().includes('Saved restore request:'),'Mismatched detail was displayed');
    await readId('f'.repeat(32));await wait(()=>text().includes('saved restore success is missing'));
    check(!text().includes('Saved restore request:'),'Wrong fingerprint displayed success');
    await readId('f'.repeat(32));await wait(()=>text().includes('Saved restore operation: failed'));
    check(text().includes('did not verify restoration'),'Saved failure displayed verified restoration');
    await readId(saved);await wait(()=>releaseDetail);
    input(panel().querySelector('input'),'d'.repeat(32));await new Promise(resolve=>setTimeout(resolve,20));
    releaseDetail();await new Promise(resolve=>setTimeout(resolve,50));check(!text().includes('STALE_DETAIL'),'Late detail survived editing its ID');
    panel().querySelector('form').requestSubmit();await wait(()=>text().includes('Saved restore operation: succeeded'));
    await click('Disconnect');await wait(()=>!text().includes('Connected to'));await connect();
    check(!text().includes('Saved restore operation: succeeded')&&panel().querySelector('input').value==='','Recovery state leaked into new connection');
    await readId(saved);await wait(()=>text().includes('Saved restore operation: succeeded'));
    await click('Find restore operations for VM 202');await wait(()=>rejectHistory);
    await click('Disconnect');await wait(()=>!text().includes('Connected to'));rejectHistory('STALE_RECOVERY_ERROR');await new Promise(resolve=>setTimeout(resolve,50));
    check(!text().includes('STALE_RECOVERY_ERROR'),'Disconnected history error leaked');
    check(calls.every(call=>!['prepare_remote_snapshot','confirm_remote_snapshot','clear_remote_snapshot','prepare_remote_snapshot_restore','confirm_remote_snapshot_restore','clear_remote_snapshot_restore'].includes(call.command)),'Recovery invoked a creation command');
    check(details===8&&histories===5,'Unexpected recovery replay or missing explicit reads');
    return {ok:true,checks:['explicit instance history/empty/error states','saved-ID and row recovery','saved/current state and guest-content distinction','uncertain/failed states without mutation controls','target/fingerprint mismatch refusal','stale edited-ID/detail and disconnect error suppressed','fresh-connection recovery with no mutation commands']};
  })().catch(error=>({ok:false,error:error.message})).then(result=>fetch('/__instance_fixture/result',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(result)}));
})();
