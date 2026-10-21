// Actual React controls, simulated IPC; no native or provider operation.
(() => {
 const id='a'.repeat(32),scope='b'.repeat(64),calls=[];let reads=0,release,reconfigured=false;
 const rows=()=>[{id,definitionName:'saved-lab',providerReference:{scopeId:scope,node:reconfigured?'new-node':'fixture-node',vmid:202},status:'registered'},{id:'f'.repeat(32),definitionName:'archived-lab',providerReference:{scopeId:scope,node:'archive-node',vmid:203},status:'deleted'}];
 async function detail(args){const yaml=`metadata:\n  name: ${args.definitionName}\n  description: <b>literal YAML</b>\nspec:\n  resources:\n    cpu: 4\n    memoryMiB: 8192\n    diskGiB: 40\n`;const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(yaml));return {id:args.target.instanceId,definitionName:args.definitionName,definitionHash:[...new Uint8Array(digest)].map(v=>v.toString(16).padStart(2,'0')).join(''),definitionRevision:3,definitionYaml:yaml,providerReference:{provider:'proxmox',scopeId:args.target.providerScopeId,node:args.target.node,vmid:args.target.vmid},status:args.target.vmid===203?'deleted':'registered'};}
 window.isTauri=true;window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
  calls.push({command,args});if(command.endsWith('_support'))return false;
  if(command==='connect_remote')return {connectionId:1,endpoint:'https://fixture.example.test/'};
  if(command==='disconnect_remote'||command==='cancel_ssh_host')return;
  if(command==='read_remote_images')return [];
  if(command==='read_remote_environments')return rows();
  if(command==='read_remote_instance_detail'){
   const row=rows().find(row=>row.id===args.target.instanceId);check(row&&args.definitionName===row.definitionName&&args.target.providerScopeId===scope&&args.target.node===row.providerReference.node&&args.target.vmid===row.providerReference.vmid,'Selected detail target changed');
   const value=await detail(args);if(args.target.vmid===203)return value;reads++;
   if(reads===2)throw 'Saved definition read unavailable';if(reads===3)value.providerReference.vmid=999;if(reads===4)value.definitionRevision=0;if(reads===5)value.definitionHash='bad';
   if(reads===6||reads===7)return new Promise(resolve=>{release=()=>resolve(value);});return value;
  }
  throw Error('Unexpected detail IPC '+command);
 }};
 function check(ok,message){if(!ok)throw Error(message);}const text=()=>document.body?.textContent||'';
 async function wait(predicate,label='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,'Detail wait expired '+label+'; '+text().slice(-600));await new Promise(resolve=>setTimeout(resolve,10));}}
 const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
 const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
 function input(node,value){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
 const panel=vmid=>document.querySelector(`[aria-label="Retained definition for VM ${vmid}"]`);
 async function connect(){await wait(()=>document.getElementById('remote-token'));input(document.getElementById('remote-endpoint'),'https://fixture.example.test');input(document.getElementById('remote-token'),'fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await wait(()=>panel(202));}
 (async()=>{
  await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));await connect();check(reads===0&&!panel(202).querySelector('pre')&&button('Read definition for VM 203'),'Automatic inspection or missing archived control');
  await click('Read definition for VM 202');await wait(()=>panel(202).querySelector('pre'));check(panel(202).textContent.includes('Definition revision: 3')&&panel(202).textContent.includes('SHA-256:'),'Saved metadata absent');check(panel(202).querySelector('pre').textContent.includes('<b>literal YAML</b>')&&!panel(202).querySelector('b'),'YAML interpreted as HTML');check(panel(202).textContent.includes('saved configuration declaration')&&panel(202).textContent.includes('does not verify current VM resources'),'Desired state presented as observation');
  await click('Refresh definition for VM 202');await wait(()=>text().includes('Saved definition read unavailable'));check(!panel(202).querySelector('pre'),'Failed refresh retained stale definition');
  for(let index=0;index<3;index++){await click('Read definition for VM 202');await wait(()=>panel(202).textContent.includes('does not match'));check(!panel(202).querySelector('pre'),'Invalid target/revision/hash accepted');}
  release=null;await click('Read definition for VM 202');await wait(()=>release);reconfigured=true;await click('Load remote workspace');await wait(()=>button('Read definition for VM 202')&&!button('Read definition for VM 202').disabled);release();await new Promise(resolve=>setTimeout(resolve,40));check(!panel(202).querySelector('pre'),'Old target reply survived workspace replacement');
  await click('Read definition for VM 203');await wait(()=>panel(203).querySelector('pre'));check(panel(203).textContent.includes('deleted · definition retained')&&panel(203).textContent.includes('archived-lab'),'Archived definition absent');await click('Hide saved definition for VM 203');check(!panel(203).querySelector('pre'),'Hide did not clear content');
  release=null;await click('Read definition for VM 202');await wait(()=>release);await click('Disconnect');await connect();release();await new Promise(resolve=>setTimeout(resolve,40));check(!panel(202).querySelector('pre'),'Disconnected reply restored content');
  check(calls.filter(call=>call.command.includes('instance_detail')).every(call=>call.command==='read_remote_instance_detail')&&!calls.some(call=>call.command.startsWith('confirm_')),'Inspection mutated work');
  await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['explicit active/archived reads','bound target/name','revision/hash metadata','literal YAML rendering','desired versus observed wording','failed refresh clears evidence','target/revision/hash refusal','workspace rebind drops stale reply','archived content and hide','reconnect drops stale content','no mutation']})});
 })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
