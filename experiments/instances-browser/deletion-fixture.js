// Real React controls and simulated IPC only: no server, provider or VM mutation.
(() => {
  const calls=[],scope="b".repeat(64),id="a".repeat(32),other="f".repeat(32);
  const environments=[{id,definitionName:"deletion-lab",providerReference:{scopeId:scope,node:"fixture-node",vmid:202},status:"registered"},{id:other,definitionName:"other-lab",providerReference:{scopeId:scope,node:"other-node",vmid:203},status:"registered"},{id:"7".repeat(32),definitionName:"retired-lab",providerReference:{scopeId:scope,node:"fixture-node",vmid:204},status:"deleted"}];
  const codes=["instance_marker","not_template","unlocked","no_pending_changes","configuration_unchanged","delete_vm_stopped","delete_not_protected","delete_scope_complete","delete_volumes_owned","delete_snapshots_complete","delete_scope_stable","delete_target_unchanged","delete_not_ha_managed","delete_no_replication_jobs","delete_management_stable","delete_provider_effect_scope"];
  let connection=0,prepares=0,attempts=0,reads=0,retained,releasePrepare,releaseRead;
  const target=()=>environments.find(environment=>environment.id===retained.instanceId);
  const queued=()=>({id:target().id===id?"e".repeat(32):"8".repeat(32),kind:"vm_delete",instanceId:target().id,status:"queued",providerTaskKnown:false,result:null,failure:null});
  const succeeded=()=>({...queued(),status:"succeeded",providerTaskKnown:true,result:{instanceId:target().id,providerScopeId:scope,node:target().providerReference.node,vmid:target().providerReference.vmid,approvalHash:"9".repeat(64),taskStatus:"succeeded",vmAbsenceVerified:true,volumesAbsenceVerified:true,definitionRetained:true,associationRetired:true}});
  function review(){const t=target().providerReference;return {instanceId:target().id,providerScopeId:scope,definitionHash:"c".repeat(64),node:t.node,vmid:t.vmid,configDigest:"d".repeat(40),scopeHash:"9".repeat(64),observation:{node:t.node,vmid:t.vmid,status:"stopped",qmpStatus:"stopped"},volumes:[{volume:`storage:vm-${t.vmid}-disk-0`,references:["live:scsi0","snapshot/before-work:scsi0"],ownerVerified:true,effect:"delete_owned_volume"},{volume:`storage:vm-${t.vmid}-fleecing`,references:["live:special/fleecing"],ownerVerified:true,effect:"fleecing_cleanup"}],snapshots:["before-work"],preservedMedia:["local:iso/debian.iso"],preservedOwnedVolumes:[`storage:vm-${t.vmid}-orphan`],effects:{version:1,removeVmConfiguration:true,removeSnapshotMetadata:true,removeVmPermissions:true,removeVmFirewall:true,cleanupIpam:true,cleanupRecordedFleecingImages:true,purgeJobs:false,destroyUnreferencedDisks:false,retainDefinitionAndHistory:true},haManaged:false,replicationJobs:[],destructive:true,definitionRetained:true,purge:false,destroyUnreferencedDisks:false,executionEnabled:true,deletionPreconditions:"passed",checks:codes.map(code=>({code,status:"passed"}))};}
  window.isTauri=true;
  window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});
    if(command.endsWith("_support"))return false;
    if(command==="connect_remote")return {connectionId:++connection,endpoint:"https://fixture.example.test/"};
    if(command==="disconnect_remote"||command==="cancel_ssh_host")return;
    if(command==="read_remote_images")return [];
    if(command==="read_remote_environments")return environments;
    if(command==="prepare_remote_deletion"){
      check(args.connectionId===connection,"Wrong preparation connection");retained=args;prepares++;
      check(Object.keys(args).sort().join(',')==='connectionId,instanceId,requestKey',"Renderer supplied a deletion plan/URL");
      const result=review();
      if(prepares===1)result.executionEnabled=false;
      if(prepares===2){result.deletionPreconditions="failed";result.checks[5].status="failed";result.observation.status="running";}
      if(prepares===3)result.vmid=999;
      if(prepares===4)result.effects.version=2;
      if(prepares===6)return new Promise(resolve=>{releasePrepare=()=>resolve({...result,node:"STALE_DELETE_REVIEW"});});
      return result;
    }
    if(command==="confirm_remote_deletion"){
      check(args.connectionId===connection && args.requestKey===retained.requestKey,"Deletion retry changed connection/key");
      check(args.confirmed===true && args.acknowledgeDataLoss===true,"Missing independent consent");
      check(Object.keys(args).sort().join(',')==='acknowledgeDataLoss,confirmed,connectionId,requestKey',"Renderer replaced native approval");
      if(++attempts===1)throw "Deletion response interrupted; retain the same request key.";
      return queued();
    }
    if(command==="read_remote_deletion_operation"){
      check(args.connectionId===connection && args.requestKey===retained.requestKey,"Wrong status connection/key");reads++;
      if(reads===1)return {...queued(),status:"running",providerTaskKnown:true};
      if(reads===2)return {...queued(),status:"uncertain",providerTaskKnown:true,failure:"provider_task_failed"};
      if(reads===3){const result=succeeded();result.result.volumesAbsenceVerified=false;return result;}
      if(reads===5){const result=succeeded();return new Promise(resolve=>{releaseRead=()=>resolve(result);});}
      return succeeded();
    }
    if(command==="clear_remote_deletion"){check(args.requestKey===retained.requestKey,"Wrong cancellation/result key");return;}
    throw Error(`Unexpected deletion fixture command: ${command}`);
  }};
  const text=()=>document.body?.textContent||'';
  function check(ok,message){if(!ok)throw Error(message);}
  async function wait(predicate,stage='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,`Deletion wait expired: ${stage}; prepares=${prepares}; attempts=${attempts}; ${text().slice(-400)}`);await new Promise(resolve=>setTimeout(resolve,10));}}
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function connect(){await wait(()=>document.getElementById('remote-token'));input('remote-endpoint','https://fixture.example.test');input('remote-token','fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await wait(()=>button('Prepare deletion for VM 202'));}
  async function prepare(){await click('Review VM deletion');await wait(()=>document.querySelector('.remote-deletion')?.getAttribute('aria-busy')==='false','preparation settled');}
  const consents=()=>[...document.querySelectorAll('.remote-deletion input[type="checkbox"]')];
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await connect();check(!button('Prepare deletion for VM 204'),'Retired identity exposed deletion');
    await click('Prepare deletion for VM 202');check(prepares===0&&attempts===0,'Selecting a VM caused preparation/submission');
    await prepare();check(text().includes('Server deletion: disabled'),'Disabled capability missing');check(button('Confirm deletion of VM 202').disabled&&consents().every(node=>!node.checked&&node.disabled),'Disabled review allowed consent');await click('Cancel deletion review');
    await prepare();check(text().includes('VM is already stopped: failed')&&button('Confirm deletion of VM 202').disabled,'Running target permitted deletion');await click('Cancel deletion review');
    await prepare();check(text().includes('Deletion review changed the selected target')&&!text().includes('Reviewed deletion:'),'Wrong VM review accepted');
    await prepare();check(!text().includes('Reviewed deletion:'),'Unknown effects accepted');
    await prepare();
    check(text().includes('storage:vm-202-disk-0')&&text().includes('snapshot/before-work:scsi0')&&text().includes('recorded fleecing cleanup'),'Reviewed destructive scope missing');
    check(text().includes('local:iso/debian.iso')&&text().includes('storage:vm-202-orphan')&&text().includes('does not purge related jobs')&&text().includes('definition and operation history are retained'),'Preservation/effects missing');
    check(consents().length===2&&consents().every(node=>!node.checked)&&button('Confirm deletion of VM 202').disabled,'Preselected/skipped consent');
    check(button('Prepare snapshot for VM 203').disabled&&button('Prepare deletion for VM 203').disabled,'Review allowed replacing mutation target');
    consents()[0].click();await new Promise(resolve=>setTimeout(resolve,20));check(button('Confirm deletion of VM 202').disabled,'Target confirmation alone allowed deletion');
    consents()[0].click();consents()[1].click();await new Promise(resolve=>setTimeout(resolve,20));check(button('Confirm deletion of VM 202').disabled,'Data-loss confirmation alone allowed deletion');
    consents()[0].click();await click('Confirm deletion of VM 202');await wait(()=>text().includes('No verified operation ID received'));
    check(consents().every(node=>node.disabled)&&!button('Cancel deletion review')&&!button('Acknowledge deletion result'),'Unknown submission was editable/clearable');
    const original=retained.requestKey;await click('Retry the same deletion request');await wait(()=>text().includes('Deletion status: queued'));
    const submits=calls.filter(call=>call.command==='confirm_remote_deletion');check(submits.length===2&&JSON.stringify(submits[0].args)===JSON.stringify(submits[1].args),'Unknown retry changed consent/key');
    check(!button('Retry the same deletion request'),'Known ID offered another POST');
    await click('Refresh deletion status');await wait(()=>text().includes('Deletion status: running'));
    await click('Refresh deletion status');await wait(()=>text().includes('Deletion status: uncertain'));
    check(text().includes('cleanup may be partial')&&!button('Acknowledge deletion result'),'Partial failure became terminal/clearable');
    await click('Refresh deletion status');await wait(()=>text().includes('Deletion success lacks verified'));
    check(!text().includes('Deletion verified:')&&text().includes('Deletion status: uncertain'),'Missing absence proof displayed success');
    await click('Refresh deletion status');await wait(()=>text().includes('Deletion verified:'));
    check(!button('Prepare deletion for VM 202')&&!button('Prepare snapshot for VM 202'),'Retired target retained active controls');
    await click('Acknowledge deletion result');await wait(()=>!text().includes('Reviewed deletion:'));
    check(button('Review VM deletion').disabled,'Old identity allowed a second review after acknowledgement');
    await click('Prepare deletion for VM 203');await click('Review VM deletion');await wait(()=>releasePrepare);
    await click('Disconnect');await connect();releasePrepare();await new Promise(resolve=>setTimeout(resolve,50));check(!text().includes('STALE_DELETE_REVIEW')&&!document.querySelector('.remote-deletion'),'Late preparation survived disconnect');
    await click('Prepare deletion for VM 203');await prepare();check(retained.requestKey!==original&&consents().every(node=>!node.checked),'Reconnect restored old key/consent');
    consents()[0].click();consents()[1].click();await click('Confirm deletion of VM 203');await wait(()=>text().includes('Deletion status: queued'));
    await click('Refresh deletion status');await wait(()=>releaseRead);await click('Disconnect');await connect();releaseRead();await new Promise(resolve=>setTimeout(resolve,50));
    check(!text().includes('Deletion verified:')&&!!button('Prepare deletion for VM 203'),'Late success retired a target on a new connection');
    check(!Object.values(localStorage).some(value=>value.includes(original)),'Consent/key persisted automatically');
    check(attempts===3,'Unexpected deletion submission');
    await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['explicit target selection','disabled/running/retargeted/unknown-effects refusal','complete deletion and preservation scope','dual unchecked consent','shared mutation target lock','identical-key unknown retry','known-ID GET-only tracking','partial cleanup retained','invalid success rejected','retired controls removed','acknowledgement','disconnect/reconnect stale prepare/read suppression']})});
  })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
