// Actual native SSH/PTY and generated application identity; no IPC mocks/personal agent.
(() => {
 const text=()=>document.body.textContent, button=label=>[...document.querySelectorAll('button')].find(node=>node.textContent===label);
 const check=(ok,message)=>{if(!ok)throw Error(message);};
 async function wait(predicate,label){const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error('Manual native SSH wait expired: '+label+'; '+text().slice(-1400));await new Promise(resolve=>setTimeout(resolve,20));}}
 const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
 function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,String(value));node.dispatchEvent(new Event('input',{bubbles:true}));}
 (async()=>{
  const config=window.__snapshotFixture;
  await wait(()=>document.getElementById('connection-mode'),'mount');const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
  await wait(()=>document.getElementById('remote-token'),'remote form');input('remote-endpoint',config.endpoint);input('remote-token',config.token);input('remote-ca',config.ca);await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();
  await click('Load remote workspace');await click('Prepare SSH for VM 202');await wait(()=>document.getElementById('ssh-host'),'manual host form');
  check(document.getElementById('ssh-host').value===''&&!document.getElementById('ssh-username'),'Endpoint/account inferred');check(text().includes('has not verified that endpoint'),'Manual endpoint qualification absent');
  input('ssh-host',config.sshHost);input('ssh-port',config.sshPort);await new Promise(resolve=>setTimeout(resolve,50));await click('Inspect host key');
  await wait(()=>text().includes(config.sshFingerprint)&&button('Trust this host key'),'unknown native key');check(button('Trust this host key').disabled&&!document.getElementById('ssh-username'),'Unknown key bypassed consent');
  document.querySelector('.remote-ssh-target input[type="checkbox"]').click();await click('Trust this host key');await wait(()=>text().includes('Saved host-key trust'),'native trust persisted');check(!document.getElementById('ssh-username'),'Trust automatically enabled auth');
  await click('Inspect host key');await wait(()=>document.getElementById('ssh-username'),'fresh trusted native handshake');check(document.getElementById('ssh-username').value==='','Account inferred from VM association');
  input('ssh-username','fixture');input('ssh-key-passphrase',config.sshPassphrase);
  document.getElementById('ssh-username').closest('form').querySelector('input[type="checkbox"]').click();
  await click('Unlock connection identity');await wait(()=>document.querySelector('input[name="ssh-identity"]'),'application identity selection');
  check(button('Open terminal').disabled,'Identity preselected');document.querySelector('input[name="ssh-identity"]').click();await click('Open terminal');
  const screen=()=>[...(document.querySelector('.xterm-rows')?.children??[])].map(row=>row.textContent).join('\n');
  const type=value=>{const node=document.querySelector('.xterm-helper-textarea');node.value=value;node.dispatchEvent(new InputEvent('input',{inputType:'insertText',data:value,bubbles:true}));};
  await wait(()=>screen().includes('fixture> '),'real shell prompt');document.querySelector('.ssh-terminal').scrollIntoView();
  type("stty -echo; printf 'MANUAL_%s_OK\\n' SSH\n");await wait(()=>screen().includes('MANUAL_SSH_OK'),'interactive stdout');
  document.querySelector('.ssh-terminal').style.height='320px';await new Promise(resolve=>setTimeout(resolve,300));
  const rows=document.querySelector('.xterm-rows').children.length;check(rows>0&&rows<24,'Renderer did not resize');
  type('stty size\n');await wait(()=>new RegExp(`\\b${rows} [1-9][0-9]*\\b`).test(screen()),'remote PTY resize');
  [...document.querySelector('.ssh-terminal').parentElement.querySelectorAll('button')].find(node=>node.textContent==='Disconnect').click();await wait(()=>text().includes('Disconnected'),'terminal closed');
  const rotated=await window.__TAURI_INTERNALS__.invoke('smoke_rotate_host');check(rotated!==config.sshFingerprint,'Owned rotation did not change key');
  await click('Inspect host key');await wait(()=>text().includes('This host key differs'),'changed native host refused');check(!button('Trust this host key')&&!document.getElementById('ssh-username'),'Changed key allowed login/trust');
  await click('Disconnect');await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
 })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:String(error)}));
})();
