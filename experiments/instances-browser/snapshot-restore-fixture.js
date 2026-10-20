// Real React controls, simulated native IPC; no server, provider or VM is contacted.
(() => {
  const calls=[],scope="b".repeat(64),id="a".repeat(32);
  const environments=[{id,definitionName:"restore-lab",providerReference:{scopeId:scope,node:"fixture-node",vmid:202},status:"registered"},{id:"f".repeat(32),definitionName:"other-lab",providerReference:{scopeId:scope,node:"other-node",vmid:203},status:"registered"}];
  const codes=["instance_marker","not_template","unlocked","no_pending_changes","configuration_unchanged","restore_vm_stopped","restore_snapshot_exists","restore_snapshot_complete","restore_snapshot_disk_only","restore_snapshot_identity","restore_snapshot_stable","restore_snapshot_storage","restore_target_unchanged"];
  let connection=0,prepares=0,attempts=0,reads=0,retained,releasePrepare,releaseRead;
  const target=()=>environments.find(environment=>environment.id===retained.instanceId);
  const observation=()=>({node:target().providerReference.node,vmid:target().providerReference.vmid,status:"stopped",qmpStatus:"stopped"});
  const snapshot=name=>({name,description:"<b>literal restore description</b>",createdAt:1700000000,includesRam:false,parent:null,providerState:null});
  const queued=()=>({id:target().id===id?"e".repeat(32):"8".repeat(32),kind:"vm_snapshot_restore",instanceId:target().id,status:"queued",providerTaskKnown:false,result:null,failure:null});
  const succeeded=()=>({...queued(),status:"succeeded",providerTaskKnown:true,result:{instanceId:target().id,providerScopeId:scope,node:target().providerReference.node,vmid:target().providerReference.vmid,taskStatus:"succeeded",snapshotName:retained.snapshotName,snapshotConfigHash:"9".repeat(64),observation:observation()}});
  function review() {return {instanceId:target().id,providerScopeId:scope,definitionHash:"c".repeat(64),node:target().providerReference.node,vmid:target().providerReference.vmid,configDigest:"d".repeat(40),request:{snapshotName:retained.snapshotName},selectedSnapshot:snapshot(retained.snapshotName),snapshotConfigHash:"9".repeat(64),observation:observation(),destructive:true,startsVm:false,executionEnabled:true,restorePreconditions:"passed",checks:codes.map(code=>({code,status:"passed"}))};}
  window.isTauri=true;
  window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});
    if(command.endsWith("_support"))return false;
    if(command==="connect_remote")return {connectionId:++connection,endpoint:"https://fixture.example.test/"};
    if(command==="disconnect_remote"||command==="cancel_ssh_host")return;
    if(command==="read_remote_images")return [];
    if(command==="read_remote_environments")return environments;
    if(command==="read_remote_snapshots") {
      const environment=environments.find(environment=>environment.id===args.instanceId);
      check(args.connectionId===connection && environment,"Wrong snapshot listing target");
      return {instanceId:environment.id,providerScopeId:scope,node:environment.providerReference.node,vmid:environment.providerReference.vmid,snapshots:[snapshot("before-work"),snapshot("other-work")]};
    }
    if(command==="prepare_remote_snapshot_restore") {
      check(args.connectionId===connection,"Wrong restore connection");retained=args;prepares++;
      check(Object.keys(args).sort().join(',')==='connectionId,instanceId,requestKey,snapshotName',"Renderer supplied restore revisions");
      const result=review();
      if(prepares===1)result.executionEnabled=false;
      if(prepares===2){result.restorePreconditions="failed";result.checks[5].status="failed";result.observation.status="running";}
      if(prepares===3)result.vmid=999;
      if(prepares===4)result.selectedSnapshot.name="wrong-snapshot";
      if(prepares===6)return new Promise(resolve=>{releasePrepare=()=>resolve({...result,request:{snapshotName:"STALE_RESTORE_REVIEW"}});});
      return result;
    }
    if(command==="confirm_remote_snapshot_restore") {
      check(args.connectionId===connection && args.requestKey===retained.requestKey && args.confirmed===true && args.acknowledgeDataLoss===true,"Restore consent/key changed");
      check(Object.keys(args).sort().join(',')==='acknowledgeDataLoss,confirmed,connectionId,requestKey',"Renderer supplied restore approval/URL");
      if(++attempts===1)throw "Restore response interrupted; retain the same request key.";
      return queued();
    }
    if(command==="read_remote_snapshot_restore_operation") {
      check(args.connectionId===connection && args.requestKey===retained.requestKey,"Wrong restore status key/connection");reads++;
      if(reads===1)return {...queued(),status:"running",providerTaskKnown:true};
      if(reads===2)return {...queued(),status:"uncertain",providerTaskKnown:true,failure:"restore_state_unconfirmed"};
      if(reads===3){const result=succeeded();result.result.snapshotConfigHash="7".repeat(64);return result;}
      if(reads===5)return new Promise(resolve=>{releaseRead=()=>resolve(succeeded());});
      return succeeded();
    }
    if(command==="clear_remote_snapshot_restore"){check(args.requestKey===retained.requestKey,"Wrong restore acknowledgement key");return;}
    throw Error(`Unexpected restore fixture command: ${command}`);
  }};
  const text=()=>document.body?.textContent||'';
  function check(ok,message){if(!ok)throw Error(message);}
  async function wait(predicate,stage='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,`Restore wait expired: ${stage}; prepares=${prepares}; attempts=${attempts}; ${text().slice(-350)}`);await new Promise(resolve=>setTimeout(resolve,10));}}
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function connect(){await wait(()=>document.getElementById('remote-token'));input('remote-endpoint','https://fixture.example.test');input('remote-token','fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');}
  async function prepare(){await click('Review snapshot restoration');await wait(()=>document.querySelector('.remote-snapshot-restore')?.getAttribute('aria-busy')==='false','prepare settled');}
  const consents=()=>[...document.querySelectorAll('.remote-snapshot-restore input[type="checkbox"]')];
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await connect();await click('List snapshots for VM 202');await click('List snapshots for VM 203');
    check(prepares===0&&attempts===0,'Listing caused restore preparation/submission');
    await click('Review restore before-work for VM 202');await wait(()=>button('Review snapshot restoration'));
    check(prepares===0&&attempts===0,'Selecting a snapshot caused a request');
    await prepare();await wait(()=>text().includes('Snapshot restoration is disabled on this server'));
    check(button('Confirm destructive restore').disabled && consents().every(node=>!node.checked&&node.disabled),'Disabled capability allowed consent');await click('Cancel restore review');
    await prepare();await wait(()=>text().includes('VM is already stopped: failed'));
    check(button('Confirm destructive restore').disabled&&attempts===0,'Running target allowed consent');await click('Cancel restore review');
    await prepare();await wait(()=>text().includes('Restore review changed the selected VM'));
    check(!text().includes('Reviewed restore:'),'Wrong VM review displayed');
    await prepare();await wait(()=>text().includes('Restore review changed the selected VM'));
    check(!text().includes('Reviewed restore:'),'Wrong selected snapshot displayed');
    await prepare();await wait(()=>text().includes('Reviewed restore:'));
    check(text().includes('restore-lab')&&text().includes('fixture-node')&&text().includes('VM 202')&&text().includes('before-work'),'Target/snapshot missing');
    check(text().includes('discards current VM data')&&text().includes('not independent backups')&&text().includes('will not shut down or start'),'Destructive/backup/power wording missing');
    check(text().includes('<b>literal restore description</b>')&&!document.querySelector('.remote-snapshot-restore b'),'Description rendered as HTML');
    check(consents().length===2&&consents().every(node=>!node.checked)&&button('Confirm destructive restore').disabled,'Consent was preselected or skipped');
    check(button('Prepare snapshot for VM 203').disabled&&button('Review restore other-work for VM 203').disabled,'Restore target can be replaced during review');
    consents()[1].click();await new Promise(resolve=>setTimeout(resolve,20));check(button('Confirm destructive restore').disabled,'Data-loss checkbox alone permitted restore');
    consents()[1].click();consents()[0].click();await new Promise(resolve=>setTimeout(resolve,20));check(button('Confirm destructive restore').disabled,'Target checkbox alone permitted restore');
    consents()[1].click();await click('Confirm destructive restore');await wait(()=>text().includes('Submission outcome unknown'));
    check(!button('Cancel restore review')&&consents().every(node=>node.disabled),'Unknown submission could be cancelled/edited');
    const original=retained.requestKey;await click('Retry same restore request');await wait(()=>text().includes('Restore operation: queued'));
    check(!button('Retry same restore request')&&attempts===2,'Known restore offered a second submission');
    const submissions=calls.filter(call=>call.command==='confirm_remote_snapshot_restore');check(JSON.stringify(submissions[0].args)===JSON.stringify(submissions[1].args),'Unknown retry changed consent/key');
    await click('Refresh restore operation');await wait(()=>text().includes('Restore operation: running'));
    await click('Refresh restore operation');await wait(()=>text().includes('Restore operation: uncertain'));
    check(!button('Acknowledge restore result'),'Uncertain restore could be cleared');
    await click('Refresh restore operation');await wait(()=>text().includes('Restore success is missing'));
    check(!text().includes('Provider rollback verified:')&&!button('Acknowledge restore result'),'Wrong fingerprint accepted as success');
    await click('Refresh restore operation');await wait(()=>text().includes('Provider rollback verified: before-work'));
    check(text().includes('Recovered guest contents and application consistency remain unverified'),'Provider result claims guest recovery');
    await click('Acknowledge restore result');await wait(()=>!text().includes('Reviewed restore:'));
    await click('Review restore other-work for VM 203');await wait(()=>text().includes('Selected snapshot: other-work'));
    await click('Review snapshot restoration');await wait(()=>releasePrepare);
    check(retained.requestKey!==original,'New restore reused old key');
    await click('Disconnect');await wait(()=>!text().includes('Connected to'));releasePrepare();await new Promise(resolve=>setTimeout(resolve,40));check(!text().includes('STALE_RESTORE_REVIEW'),'Late review survived disconnect');
    await connect();await click('List snapshots for VM 203');await click('Review restore other-work for VM 203');await prepare();await wait(()=>text().includes('Reviewed restore:'));
    check(consents().every(node=>!node.checked),'Fresh connection restored consent');consents().forEach(node=>node.click());await click('Confirm destructive restore');await wait(()=>text().includes('Restore operation: queued'));
    await click('Refresh restore operation');await wait(()=>releaseRead);await click('Disconnect');await wait(()=>!text().includes('Connected to'));releaseRead();await new Promise(resolve=>setTimeout(resolve,40));
    check(!text().includes('Provider rollback verified:')&&!document.querySelector('.remote-snapshot-restore'),'Late status survived disconnect');
    check(calls.filter(call=>call.command==='confirm_remote_snapshot_restore').length===3,'Unexpected restore replay');
    check(!calls.some(call=>['prepare_remote_snapshot','confirm_remote_snapshot'].includes(call.command)),'Restore used snapshot creation command');
    return {ok:true,checks:['explicit listing selection and destructive VM/snapshot review','independent unchecked target/data-loss consent','disabled/running/mismatched review refusal','escaped description and backup/power limits','same-key unknown-response retry','known GET-only uncertain/task-bound status and invalid-success refusal','cross-target/creation exclusion and terminal acknowledgement','fresh connection clears consent; late review/status suppressed']};
  })().catch(error=>({ok:false,error:error.message})).then(result=>fetch('/__instance_fixture/result',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(result)}));
})();
