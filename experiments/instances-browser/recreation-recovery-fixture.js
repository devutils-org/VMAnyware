// Actual React controls, simulated read-only IPC; no provider or VM mutation.
(() => {
  const id='a'.repeat(32),op='e'.repeat(32),target='f'.repeat(32),scope='b'.repeat(64),hash='c'.repeat(64),planHash='d'.repeat(64),calls=[];
  let connection=0,loads=0,mode='success',release,releaseHistory,holdHistory=false;
  const environments=[{id,definitionName:'recreation-lab',providerReference:{scopeId:scope,node:'fixture-node',vmid:202},status:'deleted'},{id:'9'.repeat(32),definitionName:'active-lab',providerReference:{scopeId:scope,node:'fixture-node',vmid:203},status:'registered'}];
  const history=()=>({sourceInstanceId:id,operations:[{id:op,instanceId:target,status:'succeeded',createdAt:'2026-10-21T00:00:00.000Z'}]});
  const saved=()=>({sourceInstanceId:id,restoresPreviousDiskContents:false,requiresNewInstanceIdentity:true,plan:{definitionName:'recreation-lab',definitionHash:hash,providerScopeId:scope,planHash,source:{templateVmid:9000,templateVersion:'fixture-v1'},target:{node:'new-node',vmid:204,storage:'local-lvm',bridge:'vmbr0',guestUser:'debian',resources:{cpu:2,memoryMiB:2048,diskGiB:8}}},operation:{id:op,kind:'vm_provision',instanceId:target,status:'succeeded',result:{instanceId:target,providerScopeId:scope,planHash,guestReady:true,guest:{ipv4:'192.0.2.42',cloudInitClean:true,cloudInitExitCode:0}}}});
  window.isTauri=true;window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});if(command.endsWith('_support'))return false;
    if(command==='connect_remote')return {connectionId:++connection,endpoint:'https://fixture.example.test/'};
    if(command==='disconnect_remote'||command==='cancel_ssh_host')return;
    if(command==='read_remote_images')return [];
    if(command==='read_remote_environments')return environments;
    if(command==='read_remote_recreation_history'){
      check(args.connectionId===connection&&args.sourceInstanceId===id,'Wrong source history');check(Object.keys(args).sort().join(',')==='connectionId,sourceInstanceId','History supplied mutation input');loads++;
      if(holdHistory)return new Promise(resolve=>{releaseHistory=()=>resolve({...history(),operations:history().operations.map(entry=>({...entry,createdAt:'STALE_HISTORY'}))});});
      if(loads===1)return {sourceInstanceId:id,operations:[]};if(loads===3)throw 'History read unavailable';return history();
    }
    if(command==='recover_remote_recreation'){
      check(args.connectionId===connection&&args.sourceInstanceId===id&&args.operationId===op,'Wrong saved detail');check(Object.keys(args).sort().join(',')==='connectionId,operationId,sourceInstanceId','Recovery supplied consent/key');
      const value=saved();if(mode==='uncertain'){value.operation.status='uncertain';value.operation.result=null;value.operation.failure='snapshot_state_unconfirmed';value.operation.message='PRIVATE_PROVIDER_OUTPUT';}if(mode==='false-success')value.operation.result.guest.cloudInitClean=false;if(mode==='wrong-source')value.sourceInstanceId='7'.repeat(32);if(mode==='late')return new Promise(resolve=>{release=()=>resolve(value);});return value;
    }
    throw Error('Unexpected recovery fixture command: '+command);
  }};
  function check(ok,message){if(!ok)throw Error(message);}const text=()=>document.body?.textContent||'';
  async function wait(predicate,stage='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,'Saved recreation wait expired: '+stage+'; '+text().slice(-500));await new Promise(resolve=>setTimeout(resolve,10));}}
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(node,value){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  const panel=()=>document.querySelector('.recreation-recovery');
  async function connect(){await wait(()=>document.getElementById('remote-token'));input(document.getElementById('remote-endpoint'),'https://fixture.example.test');input(document.getElementById('remote-token'),'fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await wait(()=>panel());}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const selector=document.getElementById('connection-mode');selector.value='remote';selector.dispatchEvent(new Event('change',{bubbles:true}));await connect();check(loads===0&&!button('Find recreations from VM 203'),'Automatic load or active-source history');
    await click('Find recreations from VM 202');await wait(()=>text().includes("No recreations in this source's recent history"));await click('Find recreations from VM 202');await click('Inspect recreation operation '+op);await wait(()=>text().includes('Saved recreation status: succeeded'));
    check(text().includes('new-node · VM 204')&&text().includes(target)&&text().includes('Current VM existence, guest readiness, address and SSH host identity have not been checked'),'Saved target/historical context missing');check(text().includes('previous disk contents and snapshots are not restored'),'Fresh disk semantics missing');
    check(!panel().querySelector('input[type="checkbox"]')&&!panel().textContent.includes('Prepare SSH'),'Saved proof restored consent or current SSH access');
    mode='uncertain';await click('Refresh saved recreation operation');await wait(()=>text().includes('Saved recreation status: uncertain'));check(text().includes('never repeats creation')&&!text().includes('Recorded guest initialization succeeded'),'Uncertain evidence presented as success');check(text().includes('snapshot task completed, but its exact disk-only snapshot is not confirmed')&&!text().includes('PRIVATE_'),'Saved recreation reason missing or leaked provider text');
    mode='false-success';await click('Inspect recreation operation '+op);await wait(()=>text().includes('Saved recreation success lacks'));check(!text().includes('Recorded guest initialization succeeded'),'False success displayed');
    mode='wrong-source';await click('Inspect recreation operation '+op);await wait(()=>text().includes('Saved recreation belongs to a different source'));mode='success';
    const before=calls.length;input(panel().querySelector('input'),'../bad');panel().querySelector('form').requestSubmit();await new Promise(resolve=>setTimeout(resolve,20));check(calls.length===before,'Invalid ID crossed IPC');
    input(panel().querySelector('input'),op);await new Promise(resolve=>setTimeout(resolve,20));panel().querySelector('form').requestSubmit();await wait(()=>text().includes('Saved recreation status: succeeded'));
    mode='late';await click('Refresh saved recreation operation');await wait(()=>release);input(panel().querySelector('input'),'7'.repeat(32));release();await new Promise(resolve=>setTimeout(resolve,40));check(!text().includes('Saved recreation status:'),'Late reply survived edited ID');mode='success';
    await click('Find recreations from VM 202');await wait(()=>text().includes('History read unavailable'));check(!text().includes("No recreations in this source's recent history"),'Error became empty history');
    holdHistory=true;await click('Find recreations from VM 202');await wait(()=>releaseHistory);await click('Disconnect');await connect();releaseHistory();await new Promise(resolve=>setTimeout(resolve,40));check(!text().includes('STALE_HISTORY')&&panel().querySelector('input').value==='','Disconnect restored saved result/ID');
    check(calls.every(call=>!['prepare_remote_recreation','confirm_remote_recreation','clear_remote_recreation'].includes(call.command)),'Recovery invoked a mutation-intent command');
    await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['retired source only','explicit empty/history lookup','scoped row/manual-ID detail','historical target and fresh-disk semantics','no consent or stale SSH handoff','uncertain/failure presentation','wrong source/false success refusal','invalid ID blocked','edited-ID stale reply discarded','error distinct from empty','fresh reconnect with no restored ID/consent','read-only IPC only']})});
  })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
