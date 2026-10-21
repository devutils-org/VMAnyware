// Real React manual SSH preparation with simulated IPC; no network/login/key writes.
(() => {
 const calls=[];let inspections=0,trusted=false,changed=false,hold=false,release,archived=false;
 const rows=()=>[202,203,204].map(vmid=>({id:String(vmid-200).repeat(32).slice(0,32),definitionName:`existing-${vmid}`,providerReference:{scopeId:'b'.repeat(64),node:'fixture-node',vmid},status:vmid===204||(vmid===202&&archived)?'deleted':'registered'}));
 window.isTauri=true;window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
  calls.push({command,args});if(command==='ssh_host_support')return true;if(command.endsWith('_support'))return false;
  if(command==='connect_remote')return {connectionId:1,endpoint:'https://fixture.example.test/'};
  if(command==='disconnect_remote'||command==='cancel_ssh_host')return;
  if(command==='read_remote_images')return [];
  if(command==='read_remote_environments')return rows();
  if(command==='inspect_ssh_host'){
   inspections++;const value={host:args.host,port:args.port,algorithm:'ssh-ed25519',fingerprint:'SHA256:manual-fixture',status:changed?'changed':trusted?'trusted':'unknown',challengeId:changed||trusted?null:`manual-${inspections}`,expiresInSeconds:120};
   if(hold)return new Promise(resolve=>{release=()=>resolve({...value,status:'unknown',challengeId:'late-challenge'});});return value;
  }
  if(command==='confirm_ssh_host'){check(args.fingerprint==='SHA256:manual-fixture'&&args.challengeId==='manual-1','Wrong manual trust evidence');trusted=true;return {host:'guest.example.test',port:2222,fingerprint:args.fingerprint};}
  throw Error('Unexpected manual SSH IPC '+command);
 }};
 const text=()=>document.body?.textContent||'';function check(ok,message){if(!ok)throw Error(message);}
 async function wait(predicate,label='controls'){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,'Manual SSH wait expired '+label+'; '+text().slice(-800));await new Promise(resolve=>setTimeout(resolve,10));}}
 const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
 const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
 function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
 async function connect(){await wait(()=>document.getElementById('remote-token'));input('remote-endpoint','https://fixture.example.test');input('remote-token','fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await click('Load remote workspace');await wait(()=>button('Prepare SSH for VM 203'));}
 async function inspect(host,port='22'){input('ssh-host',host);input('ssh-port',port);await new Promise(resolve=>setTimeout(resolve,20));document.getElementById('ssh-host').form.requestSubmit();}
 (async()=>{
  await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));await connect();
  check(!button('Prepare SSH for VM 204')&&inspections===0,'Archived SSH or automatic network action');await click('Prepare SSH for VM 202');await wait(()=>document.getElementById('ssh-host'));
  check(document.getElementById('ssh-host').value===''&&document.getElementById('ssh-port').value==='22'&&!document.getElementById('ssh-username'),'Address/account inferred');check(text().includes('SSH preparation for existing-202')&&text().includes('has not verified that endpoint'),'Manual target context missing');
  await inspect('https://guest.example.test');await wait(()=>text().includes('without a URL'));check(inspections===0,'Invalid URL crossed IPC');
  await inspect('guest.example.test','2222');await wait(()=>button('Trust this host key'));check(button('Trust this host key').disabled&&!document.getElementById('ssh-username'),'Unknown host allowed authentication/implicit trust');
  document.querySelector('.remote-ssh-target input[type="checkbox"]').click();await click('Trust this host key');await wait(()=>text().includes('Saved host-key trust'));check(inspections===1&&!document.getElementById('ssh-username'),'Trust auto-inspected/logged in');
  await click('Inspect host key');await wait(()=>document.getElementById('ssh-username'));check(document.getElementById('ssh-username').value===''&&inspections===2,'Account defaulted or fresh verification skipped');
  await click('Prepare SSH for VM 203');await wait(()=>document.getElementById('ssh-host').value==='');check(!document.getElementById('ssh-username')&&inspections===2,'Reselection restored auth or inspected automatically');
  changed=true;await inspect('other.example.test');await wait(()=>text().includes('This host key differs'));check(!button('Trust this host key')&&!document.getElementById('ssh-username'),'Changed key allowed trust/login');
  changed=false;hold=true;await inspect('late.example.test');await wait(()=>release);await click('Prepare SSH for VM 202');await wait(()=>document.getElementById('ssh-host').value==='');release();await wait(()=>calls.some(call=>call.command==='cancel_ssh_host'&&call.args.challengeId==='late-challenge'));check(!text().includes('Observed host key'),'Late inspection survived reselection');
  hold=false;trusted=false;await inspect('selected.example.test');await wait(()=>button('Trust this host key'));archived=true;await click('Load remote workspace');await wait(()=>!document.querySelector('.remote-ssh-target'));check(!button('Prepare SSH for VM 202')&&calls.some(call=>call.command==='cancel_ssh_host'&&call.args.challengeId==='manual-5'),'Archival kept old trust challenge');
  await click('Prepare SSH for VM 203');await wait(()=>document.getElementById('ssh-host'));await click('Disconnect');await connect();check(!document.querySelector('.remote-ssh-target'),'Reconnect restored manual context');
  check(!calls.some(call=>['check_ssh_authentication','start_ssh_session','unlock_app_identity','confirm_remote_lifecycle'].includes(call.command)),'Preparation unlocked/logged in/mutated');
  const first=calls.find(call=>call.command==='inspect_ssh_host');check(first.args.host==='guest.example.test'&&first.args.port===2222,'Manual endpoint replaced by provider data');
  await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['registered-only manual entry','blank address/account and explicit port','no automatic handshake/login','invalid URL blocked','explicit unknown-key trust and fresh verification','changed-key refusal','reselection clears auth context','stale inspection cancelled','archival clears pending challenge','reconnect resets manual context','no unlock/auth/session/mutation']})});
 })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
