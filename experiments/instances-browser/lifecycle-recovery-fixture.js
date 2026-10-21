// Actual React recovery controls; simulated IPC, no native/server/provider activity.
(() => {
  const id='a'.repeat(32),op='e'.repeat(32),calls=[];
  const environments=[{id,definitionName:'lifecycle-lab',providerReference:{scopeId:'b'.repeat(64),node:'current-node',vmid:202},status:'registered'},{id:'f'.repeat(32),definitionName:'archived-lab',providerReference:{scopeId:'b'.repeat(64),node:'current-node',vmid:203},status:'deleted'}];
  let loads=0,mode='success',release,releaseHistory,holdHistory=false,connection=0;
  const entry=()=>({id:op,action:'start',status:'succeeded',failure:null,providerTaskKnown:true,createdAt:'2026-10-21T00:00:00Z'});
  function saved(){return {action:'start',submissionStarted:true,target:{instanceId:id,providerScopeId:'c'.repeat(64),definitionHash:'d'.repeat(64),node:'saved-node',vmid:204},operation:{id:op,kind:'vm_start',instanceId:id,status:'succeeded',providerTaskKnown:true,failure:null,result:{action:'start',instanceId:id,providerScopeId:'c'.repeat(64),taskStatus:'succeeded',observation:{node:'saved-node',vmid:204,status:'running'}}}};}
  window.isTauri=true;
  window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});if(command.endsWith('_support'))return false;
    if(command==='connect_remote')return {connectionId:++connection,endpoint:'https://fixture.example.test/'};
    if(command==='disconnect_remote'||command==='cancel_ssh_host')return;
    if(command==='read_remote_images')return [];
    if(command==='read_remote_environments')return environments;
    if(command==='read_remote_lifecycle_history'){
      check(args.instanceId===id,'History scope changed');loads++;
      if(holdHistory)return new Promise(resolve=>{releaseHistory=()=>resolve({instanceId:id,operations:[{...entry(),id:'7'.repeat(32)}]});});
      if(loads===1)return {instanceId:id,operations:[]};
      if(loads===3)throw 'History read unavailable';
      return {instanceId:id,operations:[entry()]};
    }
    if(command==='recover_remote_lifecycle'){
      check(args.instanceId===id&&args.operationId===op,'Detail scope/ID changed');check(!('confirmed'in args)&&!('requestKey'in args),'Recovery passed consent/key');
      const value=saved();
      if(mode==='uncertain'){value.operation.status='uncertain';value.operation.result=null;value.operation.failure='power_state_unconfirmed';}
      if(mode==='false-success')value.operation.result.observation.status='stopped';
      if(mode==='wrong-instance')value.target.instanceId='7'.repeat(32);
      if(mode==='shutdown'){value.action='shutdown';value.operation.kind='vm_shutdown';value.operation.result.action='shutdown';value.operation.result.observation.status='stopped';}
      if(mode==='late')return new Promise(resolve=>{release=()=>resolve(value);});return value;
    }
    throw Error('Unexpected lifecycle recovery IPC '+command);
  }};
  function check(ok,message){if(!ok)throw Error(message);}const text=()=>document.body?.textContent||'';
  async function wait(predicate,label='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,'Lifecycle recovery wait expired: '+label+'; '+text().slice(-700));await new Promise(resolve=>setTimeout(resolve,10));}}
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(node,value){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  const panel=()=>document.querySelector('.lifecycle-recovery');
  async function connect(){await wait(()=>document.getElementById('remote-token'));input(document.getElementById('remote-endpoint'),'https://fixture.example.test');input(document.getElementById('remote-token'),'fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await wait(()=>panel());}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const selector=document.getElementById('connection-mode');selector.value='remote';selector.dispatchEvent(new Event('change',{bubbles:true}));await connect();
    check(loads===0&&button('Find lifecycle operations for VM 203'),'Automatic load or archived history absent');
    await click('Find lifecycle operations for VM 202');await wait(()=>text().includes("No lifecycle operations in this instance's recent history"));
    await click('Find lifecycle operations for VM 202');await click('Inspect lifecycle operation '+op);await wait(()=>text().includes('Saved lifecycle status: succeeded'));
    check(panel().textContent.includes('saved-node · VM 204')&&panel().textContent.includes('historical evidence')&&panel().textContent.includes('Current VM existence, power, guest readiness and SSH host identity have not been checked'),'Historical target/context absent');
    check(!panel().querySelector('input[type="checkbox"]')&&![...panel().querySelectorAll('button')].some(node=>/confirm|SSH|retry/i.test(node.textContent)),'History restored consent or action');
    mode='uncertain';await click('Refresh saved lifecycle operation');await wait(()=>text().includes('Saved lifecycle status: uncertain'));check(panel().textContent.includes('never repeats the action')&&!text().includes('Recorded task and power state were verified'),'Unknown record became success');
    mode='false-success';await click('Inspect lifecycle operation '+op);await wait(()=>text().includes('Saved lifecycle success lacks'));check(!text().includes('Recorded task and power state were verified'),'False success shown');
    mode='wrong-instance';await click('Inspect lifecycle operation '+op);await wait(()=>text().includes('different instance or action'));
    const before=calls.length;input(panel().querySelector('input'),'../bad');panel().querySelector('form').requestSubmit();await new Promise(resolve=>setTimeout(resolve,20));check(calls.length===before,'Invalid ID crossed IPC');
    mode='shutdown';input(panel().querySelector('input'),op);await new Promise(resolve=>setTimeout(resolve,20));panel().querySelector('form').requestSubmit();await wait(()=>text().includes('Saved lifecycle action: graceful shutdown'));check(panel().textContent.includes('were verified: stopped'),'Shutdown evidence absent');
    mode='late';await click('Refresh saved lifecycle operation');await wait(()=>release);input(panel().querySelector('input'),'7'.repeat(32));release();await new Promise(resolve=>setTimeout(resolve,40));check(!text().includes('Saved lifecycle status:'),'Edited ID accepted late detail');
    await click('Find lifecycle operations for VM 202');await wait(()=>text().includes('History read unavailable'));check(!text().includes("No lifecycle operations in this instance's recent history"),'History failure became empty success');
    holdHistory=true;await click('Find lifecycle operations for VM 202');await wait(()=>releaseHistory);await click('Disconnect');await connect();releaseHistory();await new Promise(resolve=>setTimeout(resolve,40));check(panel().querySelector('input').value===''&&!text().includes('Inspect lifecycle operation '+'7'.repeat(32)),'Reconnect restored ID or late history');
    check(calls.filter(call=>call.command.includes('lifecycle')).every(call=>['read_remote_lifecycle_history','recover_remote_lifecycle'].includes(call.command)),'Recovery prepared or submitted lifecycle intent');
    await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['registered and archived history controls','explicit empty/history discovery','row and manual-ID reads','historical saved target','no consent/retry/SSH action','uncertain presentation','false success and wrong instance refusal','invalid ID blocked','shutdown evidence','edited-ID stale reply discarded','history failure distinct from empty','reconnect resets evidence and ID','read-only recovery IPC']})});
  })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
