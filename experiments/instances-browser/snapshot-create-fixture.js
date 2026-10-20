// Actual React controls with simulated IPC; no native identity, server or VM.
(() => {
  const calls=[], id="a".repeat(32), scope="b".repeat(64);
  const environments=[{id,definitionName:"snapshot-lab",providerReference:{scopeId:scope,node:"fixture-node",vmid:202},status:"registered"},{id:"f".repeat(32),definitionName:"other-lab",providerReference:{scopeId:scope,node:"other-node",vmid:203},status:"registered"}];
  const codes=["instance_marker","not_template","unlocked","no_pending_changes","configuration_unchanged","snapshot_supported","snapshot_name_available","snapshot_target_unchanged","snapshot_tree_complete"];
  let prepares=0, attempts=0, reads=0, retained, release;
  function review(args) {const target=environments.find(target=>target.id===args.instanceId);return {instanceId:target.id,providerScopeId:scope,definitionHash:"c".repeat(64),node:target.providerReference.node,vmid:target.providerReference.vmid,configDigest:"d".repeat(40),request:{name:args.name,description:args.description},includesRam:false,executionEnabled:true,creationPreconditions:"passed",checks:codes.map(code=>({code,status:"passed"}))};}
  const queued=()=>({id:"e".repeat(32),kind:"vm_snapshot_create",instanceId:id,status:"queued",providerTaskKnown:false,result:null,failure:null});
  window.isTauri=true;
  window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});
    if(command.endsWith("_support"))return false;
    if(command==="connect_remote")return {connectionId:1,endpoint:"https://fixture.example.test/"};
    if(command==="disconnect_remote" || command==="cancel_ssh_host")return;
    if(command==="read_remote_images")return [];
    if(command==="read_remote_environments")return environments;
    if(command==="prepare_remote_snapshot") {
      check(args.connectionId===1,"Wrong snapshot connection");prepares++;
      retained=args;const result=review(args);
      if(prepares===1)result.executionEnabled=false;
      if(prepares===2){result.creationPreconditions="failed";result.checks[5].status="failed";}
      if(prepares===3)result.vmid=999;
      if(prepares===5)return new Promise(resolve=>{release=()=>resolve({...result,request:{name:"STALE_SNAPSHOT_REVIEW",description:""}});});
      return result;
    }
    if(command==="confirm_remote_snapshot") {
      check(args.connectionId===1 && args.requestKey===retained.requestKey && args.confirmed===true,"Retained consent/key changed");
      check(Object.keys(args).sort().join(',')==='confirmed,connectionId,requestKey',"Renderer supplied new approval fields");
      if(++attempts===1)throw "Snapshot response interrupted; retain the same request key.";
      return queued();
    }
    if(command==="read_remote_snapshot_operation") {
      check(args.requestKey===retained.requestKey,"Wrong status request key");reads++;
      if(reads===1)return {...queued(),status:"running",providerTaskKnown:true};
      if(reads===2)return {...queued(),status:"uncertain",providerTaskKnown:true,failure:"snapshot_state_unconfirmed"};
      return {...queued(),status:"succeeded",providerTaskKnown:true,result:{instanceId:id,providerScopeId:scope,node:"fixture-node",vmid:202,taskStatus:"succeeded",snapshot:{name:retained.name,description:retained.description,includesRam:false,providerState:null}}};
    }
    if(command==="clear_remote_snapshot"){check(args.requestKey===retained.requestKey,"Wrong acknowledgement key");return;}
    throw `Unexpected snapshot-create command: ${command}`;
  }};
  const text=()=>document.body?.textContent || '';
  function check(ok,message){if(!ok)throw Error(message);}
  async function wait(predicate,stage='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,`Snapshot create wait expired: ${stage}; prepares=${prepares}; attempts=${attempts}`);await new Promise(resolve=>setTimeout(resolve,10));}}
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function prepare(){await click('Review snapshot creation');await wait(()=>!document.querySelector('.remote-snapshot-create').getAttribute('aria-busy') || document.querySelector('.remote-snapshot-create').getAttribute('aria-busy')==='false','prepare settled');}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'));
    input('remote-endpoint','https://fixture.example.test');input('remote-token','fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();
    await click('Load remote workspace');await click('Prepare snapshot for VM 202');
    await wait(()=>document.getElementById('snapshot-name'));check(prepares===0 && attempts===0,'Selecting target triggered a request');
    input('snapshot-description','<b>literal snapshot description</b>');await new Promise(resolve=>setTimeout(resolve,20));
    await prepare();await wait(()=>text().includes('Snapshot creation is disabled on this server'));
    check(button('Confirm snapshot creation').disabled && attempts===0,'Disabled server allowed confirmation');await click('Cancel snapshot review');
    await prepare();await wait(()=>text().includes('Snapshot support: failed'));
    check(button('Confirm snapshot creation').disabled && attempts===0,'Failed preflight allowed confirmation');await click('Cancel snapshot review');
    await prepare();await wait(()=>text().includes('Snapshot review changed the selected target'));
    check(!text().includes('Reviewed snapshot:'),'Mismatched review was displayed');
    await prepare();await wait(()=>text().includes('Reviewed snapshot:'));
    check(text().includes('snapshot-lab') && text().includes('fixture-node') && text().includes('VM 202'),'Reviewed target absent');
    check(text().includes('not independent backups') && text().includes('RAM excluded'),'Snapshot/backup semantics unclear');
    check(text().includes('<b>literal snapshot description</b>') && !document.querySelector('.remote-snapshot-create b'),'Description rendered as HTML');
    check(button('Confirm snapshot creation').disabled && attempts===0,'Review submitted or skipped consent');
    check(document.getElementById('snapshot-name').disabled && button('Prepare snapshot for VM 203').disabled,'Review target/input can be replaced');
    document.querySelector('.remote-snapshot-create input[type="checkbox"]').click();await click('Confirm snapshot creation');
    await wait(()=>text().includes('Submission outcome unknown'));check(!button('Cancel snapshot review'),'Unknown submission offered cancellation');
    const original=retained.requestKey;await click('Retry same snapshot request');await wait(()=>text().includes('Snapshot operation: queued'));
    check(attempts===2 && retained.requestKey===original && !button('Retry same snapshot request'),'Retry changed intent or known operation allowed POST');
    const submissions=calls.filter(call=>call.command==='confirm_remote_snapshot');check(JSON.stringify(submissions[0].args)===JSON.stringify(submissions[1].args),'Retry arguments changed');
    await click('Refresh snapshot operation');await wait(()=>text().includes('Snapshot operation: running'));
    await click('Refresh snapshot operation');await wait(()=>text().includes('Snapshot operation: uncertain'));
    check(!button('Acknowledge snapshot result'),'Uncertain operation could be acknowledged');
    await click('Refresh snapshot operation');await wait(()=>text().includes('Snapshot creation verified: before-work'));
    await click('Acknowledge snapshot result');await wait(()=>!text().includes('Reviewed snapshot:'));
    await click('Prepare snapshot for VM 203');await wait(()=>text().includes('Target: other-lab'));
    input('snapshot-name','other-work');await new Promise(resolve=>setTimeout(resolve,20));await click('Review snapshot creation');await wait(()=>release);
    check(retained.requestKey!==original,'A new snapshot reused the previous key');
    await click('Disconnect');await wait(()=>!text().includes('Connected to'));
    release();await new Promise(resolve=>setTimeout(resolve,50));check(!text().includes('STALE_SNAPSHOT_REVIEW'),'Late snapshot preparation survived disconnect');
    check(calls.filter(call=>call.command==='confirm_remote_snapshot').length===2,'Unexpected submission replay');
    return {ok:true,checks:['explicit target/review/consent','disabled/failed/mismatched preflight refusal','escaped description and disk-only/backup wording','retained same-key retry after lost reply','GET-only queued/running/uncertain/success controls','terminal acknowledgement and target exclusion','stale preparation suppressed after disconnect']};
  })().catch(error=>({ok:false,error:error.message})).then(result=>fetch('/__instance_fixture/result',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(result)}));
})();
