// Real React interaction, simulated IPC only; no server or VM mutation.
(() => {
  const id = 'a'.repeat(32), scope = 'b'.repeat(64), calls = [];
  const environments = [202,203,204].map((vmid,index) => ({id:index===0?id:String(index).repeat(32),definitionName:`lifecycle-${vmid}`,providerReference:{scopeId:scope,node:'fixture-node',vmid},status:vmid===204?'deleted':'registered'}));
  const codes = ['instance_marker','not_template','unlocked','no_pending_changes','configuration_unchanged'];
  let prepares=0, attempts=0, reads=0, retained, release, connection=0;
  function review(args) {
    return {target:args.target,action:args.action,preflight:{instanceId:args.target.instanceId,providerScopeId:scope,definitionHash:'c'.repeat(64),expectedMarker:`vmanyware-${args.target.instanceId}`,configDigest:'d'.repeat(40),observation:{node:'fixture-node',vmid:202,status:args.action==='start'?'stopped':'running',qmpStatus:args.action==='start'?null:'running'},checks:codes.map(code=>({code,status:'passed'})),startPreconditions:args.action==='start'?'passed':'failed',shutdownPreconditions:args.action==='shutdown'?'passed':'failed',executionEnabled:true}};
  }
  const operation = (status='queued') => ({id:'e'.repeat(32),kind:retained.action==='start'?'vm_start':'vm_shutdown',instanceId:id,status,providerTaskKnown:status!=='queued',failure:status==='uncertain'?'power_state_unconfirmed':null,result:status==='succeeded'?{action:retained.action,instanceId:id,providerScopeId:scope,taskStatus:'succeeded',observation:{node:'fixture-node',vmid:202,status:retained.action==='start'?'running':'stopped',qmpStatus:null}}:null});
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = {invoke:async(command,args={}) => {
    calls.push({command,args});
    if(command.endsWith('_support')) return false;
    if(command==='connect_remote') return {connectionId:++connection,endpoint:'https://fixture.example.test/'};
    if(command==='disconnect_remote'||command==='cancel_ssh_host')return;
    if(command==='read_remote_images')return [];
    if(command==='read_remote_environments')return environments;
    if(command==='prepare_remote_lifecycle') {
      check(args.connectionId===connection && args.target.instanceId===id && args.target.providerScopeId===scope && args.target.node==='fixture-node' && args.target.vmid===202,'Target changed before IPC');
      prepares++;retained=args;const value=review(args);
      if(prepares===1)value.preflight.executionEnabled=false;
      if(prepares===2){value.preflight.startPreconditions='unknown';value.preflight.checks[0].status='unknown';}
      if(prepares===3)value.target={...value.target,vmid:999};
      if(prepares===7)return new Promise(resolve=>{release=()=>resolve(value);});
      return value;
    }
    if(command==='clear_remote_lifecycle'){check(args.requestKey===retained.requestKey,'Wrong clear key');return;}
    if(command==='confirm_remote_lifecycle'){
      check(args.requestKey===retained.requestKey && args.confirmed===true,'Consent/key changed');
      check(Object.keys(args).sort().join(',')==='confirmed,connectionId,requestKey','Renderer supplied new approval');
      if(prepares===4)throw 'The lifecycle review expired. Prepare and review again.';
      if(++attempts===1)throw 'Reply unavailable; retain this lifecycle request key.';
      return operation();
    }
    if(command==='read_remote_lifecycle_operation'){
      check(args.requestKey===retained.requestKey,'Wrong status key');reads++;
      if(reads===1)return operation('uncertain');
      if(reads===2){const value=operation('succeeded');value.result.providerScopeId='f'.repeat(64);return value;}
      return operation('succeeded');
    }
    throw Error(`Unexpected IPC ${command}`);
  }};
  function check(value,message){if(!value)throw Error(message);}
  const text=()=>document.body.textContent;
  const button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
  async function wait(test,label='condition'){const end=Date.now()+5000;while(!test()){if(Date.now()>end)throw Error(`Timed out: ${label}\n${text().slice(-2500)}`);await new Promise(resolve=>setTimeout(resolve,10));}}
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  async function connect(){await wait(()=>document.getElementById('remote-token'));input('remote-endpoint','https://fixture.example.test');input('remote-token','fixture-only-access-token-0123456789abcdef');await new Promise(resolve=>setTimeout(resolve,20));document.querySelector('.remote-connection form').requestSubmit();await wait(()=>text().includes('Connected to'));}
  const consent=()=>document.getElementById('remote-lifecycle-confirm');
  async function prepare(action='start'){await click(`Review remote ${action}`);await wait(()=>document.querySelector('.remote-lifecycle')?.getAttribute('aria-busy')==='false','review settled');}
  (async()=>{
    await wait(()=>document.getElementById('connection-mode'));const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await connect();await click('Load remote workspace');await click('Prepare start for VM 202');
    check(!button('Prepare start for VM 204') && prepares===0 && attempts===0,'Deleted VM/action selection caused an action');
    await prepare();check(button('Confirm remote start').disabled && !consent().checked,'Disabled execution allowed confirmation');await click('Cancel lifecycle review');
    await prepare();check(button('Confirm remote start').disabled && text().includes('Instance marker matches: unknown'),'Unknown review allowed confirmation');await click('Cancel lifecycle review');
    await prepare();check(text().includes('Lifecycle review changed')&&!text().includes('Reviewed lifecycle:'),'Changed target review accepted');
    await prepare();const old=retained.requestKey;
    check(button('Confirm remote start').disabled && !consent().checked,'Review skipped consent');
    check(button('Prepare shutdown for VM 202').disabled&&button('Prepare start for VM 203').disabled&&button('Prepare deletion for VM 202').disabled,'Mutation target not locked');
    consent().click();await click('Confirm remote start');await wait(()=>text().includes('The lifecycle review expired'));check(button('Cancel lifecycle review')&&!consent().checked&&!text().includes('No verified lifecycle operation ID received'),'Expired unsubmitted review became unknown work');await click('Cancel lifecycle review');await prepare();check(!consent().checked&&retained.requestKey!==old,'Consent/key survived new review');
    consent().click();await click('Confirm remote start');await wait(()=>text().includes('No verified lifecycle operation ID received'));
    check(!button('Cancel lifecycle review')&&consent().disabled,'Unknown action offered cancellation/edit');
    await click('Retry same lifecycle request');await wait(()=>text().includes('Lifecycle operation: queued'));
    const posts=calls.filter(call=>call.command==='confirm_remote_lifecycle'&&call.args.requestKey!==old);check(posts.length===2&&JSON.stringify(posts[0].args)===JSON.stringify(posts[1].args),'Retry changed arguments');check(!button('Retry same lifecycle request'),'Known ID offered another submit');
    await click('Refresh lifecycle operation');await wait(()=>text().includes('Lifecycle operation: uncertain'));check(!button('Acknowledge lifecycle result'),'Uncertain action clearable');
    await click('Refresh lifecycle operation');await wait(()=>text().includes('Lifecycle success lacks matching'));check(text().includes('Lifecycle operation: uncertain'),'Unrelated success replaced retained evidence');
    await click('Refresh lifecycle operation');await wait(()=>text().includes('Lifecycle operation: succeeded'));check(text().includes('Guest readiness and SSH access are not verified'),'Power success claimed SSH readiness');
    check(![...document.querySelectorAll('.remote-lifecycle button')].some(node=>node.textContent.includes('SSH')),'Lifecycle offered implicit SSH');
    await click('Acknowledge lifecycle result');await click('Prepare shutdown for VM 202');await prepare('shutdown');check(text().includes('unsaved work may be lost')&&!consent().checked,'Shutdown warning/consent absent');
    consent().click();await click('Confirm remote shutdown');await wait(()=>text().includes('Lifecycle operation: queued'));await click('Refresh lifecycle operation');await wait(()=>text().includes('power state verified: stopped'));await click('Acknowledge lifecycle result');
    await click('Review remote shutdown');await wait(()=>release);await click('Disconnect');await connect();release();await new Promise(resolve=>setTimeout(resolve,50));check(!text().includes('Reviewed lifecycle:')&&!consent(),'Stale review/consent survived reconnect');
    check(attempts===3,'Unexpected automatic submission');
    await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:true,checks:['remote start/shutdown selection','disabled/unknown/changed-target refusal','unchecked renewable consent','cross-action target lock','identical unknown retry','known operation reads','unrelated success rejected','uncertain work retained','power vs SSH readiness','graceful shutdown warning','terminal acknowledgement','stale reconnect suppression']})});
  })().catch(async error=>{await fetch('/__instance_fixture/result',{method:'POST',body:JSON.stringify({ok:false,error:error.stack||String(error)})});});
})();
