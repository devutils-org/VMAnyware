// Real React, simulated IPC; no provider or VM read/mutation.
(() => {
 const id='a'.repeat(32),scope='b'.repeat(64),calls=[];let reads=0,release;
 const environments=[{id,definitionName:'power-lab',providerReference:{scopeId:scope,node:'fixture-node',vmid:202},status:'registered'},{id:'f'.repeat(32),definitionName:'archived-lab',providerReference:{scopeId:scope,node:'fixture-node',vmid:203},status:'deleted'}];
 const value=()=>({instanceId:id,providerScopeId:scope,observation:{node:'fixture-node',vmid:202,status:'running',qmpStatus:'paused'}});
 window.isTauri=true;window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
  calls.push({command,args});if(command.endsWith('_support'))return false;
  if(command==='connect_remote')return {connectionId:1,endpoint:'https://fixture.example.test/'};
  if(command==='disconnect_remote'||command==='cancel_ssh_host'||command==='clear_remote_lifecycle')return;
  if(command==='read_remote_images')return [];
  if(command==='read_remote_environments')return environments;
  if(command==='read_remote_instance_status'){
   check(JSON.stringify(args.target)===JSON.stringify({instanceId:id,providerScopeId:scope,node:'fixture-node',vmid:202}),'Selected target changed');reads++;
   if(reads===2)throw 'Power read unavailable';
   if(reads===3){const result=value();result.observation.vmid=999;return result;}
   if(reads===4||reads===6)return new Promise(resolve=>{release=()=>resolve(value());});
   if(reads===5){const result=value();result.observation.status='stopped';result.observation.qmpStatus=null;return result;}return value();
  }
  if(command==='prepare_remote_lifecycle')return {target:args.target,action:args.action,preflight:{instanceId:id,providerScopeId:scope,definitionHash:'c'.repeat(64),expectedMarker:`vmanyware-${id}`,configDigest:'d'.repeat(40),observation:{node:'fixture-node',vmid:202,status:'stopped',qmpStatus:null},checks:['instance_marker','not_template','unlocked','no_pending_changes','configuration_unchanged'].map(code=>({code,status:'passed'})),startPreconditions:'passed',shutdownPreconditions:'failed',executionEnabled:true}};
  throw Error('Unexpected IPC '+command);
 }};
 const text=()=>document.body?.textContent||'';function check(ok,message){if(!ok)throw Error(message);}
 async function wait(predicate,label='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,'Power wait expired '+label+'; '+text().slice(-600));await new Promise(resolve=>setTimeout(resolve,10));}}
 const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
 const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
 function input(node,value){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
 const panel=()=>document.querySelector('.remote-instance-status');
 async function connect(){await wait(()=>document.getElementById('remote-token'));input(document.getElementById('remote-endpoint'),'https://fixture.example.test');input(document.getElementById('remote-token'),'fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await wait(()=>panel());}
 (async()=>{
  await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));await connect();
  check(reads===0&&!button('Read power for VM 203')&&panel().textContent.includes('Power state not observed'),'Automatic read, archived read or false initial power');
  await click('Read power for VM 202');await wait(()=>panel().textContent.includes('Observed power: running'));check(panel().textContent.includes('QMP: paused')&&panel().querySelector('time')?.dateTime&&panel().textContent.includes('may already be stale'),'Observation time/state/stale warning missing');check(panel().textContent.includes('Power does not verify ownership, guest readiness'),'Power implied readiness');
  await click('Refresh power for VM 202');await wait(()=>panel().textContent.includes('Power read unavailable'));check(!panel().textContent.includes('Observed power:')&&!panel().querySelector('time'),'Failed read kept old success');
  await click('Read power for VM 202');await wait(()=>panel().textContent.includes('does not match'));check(!panel().textContent.includes('Observed power:'),'Other target accepted');
  await click('Read power for VM 202');await wait(()=>reads===4&&release);await click('Prepare start for VM 202');await click('Review remote start');await wait(()=>text().includes('Reviewed lifecycle:'));release();await new Promise(resolve=>setTimeout(resolve,40));check(!panel().textContent.includes('Observed power:')&&button('Read power for VM 202').disabled,'Late read survived mutation review');await click('Cancel lifecycle review');
  await click('Read power for VM 202');await wait(()=>panel().textContent.includes('Observed power: stopped'));check(panel().textContent.includes('QMP: unknown'),'Missing QMP became ready');
  release=null;await click('Refresh power for VM 202');await wait(()=>reads===6&&release);await click('Disconnect');await connect();release();await new Promise(resolve=>setTimeout(resolve,40));check(!panel().textContent.includes('Observed power:')&&!panel().querySelector('time'),'Disconnected read restored observation');
  check(!calls.some(call=>call.command==='confirm_remote_lifecycle'||call.command.includes('ssh_session')),'Power read mutated or connected SSH');
  await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['explicit registered-only read','exact target binding','running/paused distinct','observation timestamp and staleness','failure clears old evidence','wrong target refused','mutation review invalidates inflight observation','stopped/unknown QMP','reconnect drops stale observation','no write or SSH']})});
 })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
