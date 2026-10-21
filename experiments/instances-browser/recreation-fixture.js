// Real React interaction with simulated IPC. No server/provider/VM mutation.
(() => {
  const calls=[],id='a'.repeat(32),newId='f'.repeat(32),scope='b'.repeat(64),hash='c'.repeat(64),planHash='d'.repeat(64);
  const codes=['template','source_disk','cloud_init_drive','storage','storage_capacity','bridge','target_vmid','template_revision','execution_permissions','guest_readiness'];
  let connection=0,prepares=0,attempts=0,reads=0,retained,release;
  const environments=[{id,definitionName:'recreation-lab',providerReference:{scopeId:scope,node:'fixture-node',vmid:202},status:'deleted'},{id:'9'.repeat(32),definitionName:'active-lab',providerReference:{scopeId:scope,node:'fixture-node',vmid:204},status:'registered'}];
  function review(){return {sourceInstanceId:id,sourceDefinitionHash:hash,sourceDefinitionRevision:1,definitionYaml:'<retained YAML literal>',plan:{definitionName:'recreation-lab',definitionHash:hash,planHash,providerScopeId:scope,creationEnabled:true,sshKeyCount:1,source:{templateVmid:9000,templateVersion:'fixture-v1'},target:{node:'fixture-node',vmid:203,storage:'local-lvm',bridge:'vmbr0',guestUser:'debian',resources:{cpu:2,memoryMiB:2048,diskGiB:8}}},restoresPreviousDiskContents:false,requiresNewInstanceIdentity:true,recreationEnabled:true,preflight:{planHash,creationEnabled:true,vmidReserved:false,checks:codes.map((code,index)=>({code,status:index<7?'passed':'unknown'}))}};}
  const queued=()=>({id:'e'.repeat(32),kind:'vm_provision',instanceId:newId,status:'queued',result:null});
  const success=()=>({...queued(),status:'succeeded',result:{instanceId:newId,providerScopeId:scope,planHash,guestReady:true,guest:{ipv4:'192.0.2.42',cloudInitExitCode:0,cloudInitClean:true}}});
  window.isTauri=true;window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});if(command.endsWith('_support'))return false;
    if(command==='connect_remote')return {connectionId:++connection,endpoint:'https://fixture.example.test/'};
    if(command==='disconnect_remote'||command==='cancel_ssh_host')return;
    if(command==='read_remote_images')return [];
    if(command==='read_remote_environments')return environments;
    if(command==='prepare_remote_recreation'){
      check(args.sourceInstanceId===id&&args.targetVmid===203,'Wrong recreation source/target');check(Object.keys(args).sort().join(',')==='connectionId,requestKey,sourceInstanceId,targetVmid','Renderer supplied YAML/plan/URL');retained=args;prepares++;const result=review();
      if(prepares===1){result.recreationEnabled=false;result.plan.creationEnabled=false;result.preflight.creationEnabled=false;}
      if(prepares===2)result.preflight.checks[6].status='failed';
      if(prepares===3)result.sourceInstanceId='7'.repeat(32);
      if(prepares===6)return new Promise(resolve=>{release=()=>resolve({...result,definitionYaml:'STALE_RECREATION'});});
      return result;
    }
    if(command==='clear_remote_recreation'){check(args.requestKey===retained.requestKey,'Clear changed request key');return;}
    if(command==='confirm_remote_recreation'){
      check(args.connectionId===connection&&args.requestKey===retained.requestKey&&args.confirmed===true,'Wrong recreation consent/key');check(Object.keys(args).sort().join(',')==='confirmed,connectionId,requestKey','Renderer replaced approval');
      if(++attempts===1)throw 'Response interrupted; retain the original request key.';return queued();
    }
    if(command==='read_remote_recreation_operation'){
      check(args.requestKey===retained.requestKey,'Wrong saved request key');reads++;
      if(reads===1)return {...queued(),status:'uncertain'};
      if(reads===2){const value=success();value.result.guest.cloudInitClean=false;return value;}
      if(reads===3)return {...queued(),instanceId:id};
      return success();
    }
    throw Error('Unexpected recreation fixture command: '+command);
  }};
  const text=()=>document.body?.textContent||'';function check(ok,message){if(!ok)throw Error(message);}
  async function wait(predicate,stage='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,'Recreation wait expired: '+stage+'; '+text().slice(-600));await new Promise(resolve=>setTimeout(resolve,10));}}
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function connect(){await wait(()=>document.getElementById('remote-token'));input('remote-endpoint','https://fixture.example.test');input('remote-token','fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await click('Prepare recreation from VM 202');input('recreation-vmid','203');await new Promise(resolve=>setTimeout(resolve,20));}
  async function prepare(){await click('Review recreation');await wait(()=>document.querySelector('.remote-recreation').getAttribute('aria-busy')==='false','review settled');}
  const consent=()=>document.getElementById('recreation-confirm');
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));await connect();
    check(!button('Prepare recreation from VM 204')&&prepares===0&&attempts===0,'Active row/selection caused recreation');
    await prepare();check(text().includes('Server recreation: disabled')&&consent().disabled&&button('Confirm recreation').disabled,'Disabled capability allowed consent');await click('Cancel recreation review');
    await prepare();check(text().includes('Target VMID was observed free: failed')&&button('Confirm recreation').disabled,'Occupied target allowed recreation');await click('Cancel recreation review');
    await prepare();check(text().includes('Recreation review changed the selected source')&&!text().includes('Reviewed recreation:'),'Changed source accepted');
    await prepare();check(!consent().checked&&button('Confirm recreation').disabled,'Preselected consent');
    check(text().includes('Previous disk contents and snapshots are not restored')&&text().includes('VM 203')&&text().includes('revision 1')&&text().includes('local-lvm')&&text().includes('<retained YAML literal>'),'Source/target/fresh disk review missing');
    check(button('Prepare deletion for VM 204').disabled&&button('Prepare snapshot for VM 204').disabled,'Review allowed replacing target');
    consent().click();await click('Cancel recreation review');await prepare();check(!consent().checked&&button('Confirm recreation').disabled,'Consent survived cancellation');
    const original=retained.requestKey;consent().click();await click('Confirm recreation');await wait(()=>text().includes('No verified operation ID received'));
    check(consent().disabled&&!button('Cancel recreation review')&&document.getElementById('recreation-vmid').disabled,'Unknown submission editable');
    await click('Retry the same recreation request');await wait(()=>text().includes('Recreation status: queued'));
    const submits=calls.filter(call=>call.command==='confirm_remote_recreation');check(submits.length===2&&JSON.stringify(submits[0].args)===JSON.stringify(submits[1].args),'Retry changed request/consent');check(!button('Retry the same recreation request'),'Known ID offered POST');
    await click('Refresh recreation status');await wait(()=>text().includes('Recreation status: uncertain'));check(!button('Acknowledge recreation result'),'Uncertain operation clearable');
    await click('Refresh recreation status');await wait(()=>text().includes('Recreation success lacks matching'));check(!button('Prepare SSH for recreated VM'),'False guest readiness offered SSH');
    await click('Refresh recreation status');await wait(()=>text().includes('distinct new instance'));check(text().includes('Recreation status: uncertain'),'Old identity reused');
    await click('Refresh recreation status');await wait(()=>text().includes('Recreation status: succeeded'));check(text().includes(newId)&&text().includes('192.0.2.42'),'New instance evidence missing');
    await click('Prepare SSH for recreated VM');await wait(()=>text().includes('SSH target: debian@192.0.2.42'));check(text().includes('independently verify its host key'),'SSH trust bypassed');
    await click('Acknowledge recreation result');await wait(()=>!text().includes('Reviewed recreation:'));
    await click('Review recreation');await wait(()=>release);await click('Disconnect');await connect();release();await new Promise(resolve=>setTimeout(resolve,50));check(!text().includes('STALE_RECREATION')&&!text().includes('Reviewed recreation:'),'Stale review survived reconnect');
    check(!document.getElementById('recreation-confirm')&&!Object.values(localStorage).some(value=>value.includes(original)),'Consent/key restored automatically');check(attempts===2,'Unexpected extra submission');
    await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['deleted source selection','disabled/occupied/changed-source refusal','retained definition and fresh disk semantics','unchecked cancellable consent','mutation target lock','identical-key retry','GET-only known status','uncertain result retained','false guest readiness/old identity rejected','SSH trust handoff','terminal acknowledgement','stale response suppression']})});
  })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
