// Real React/native HTTPS/API/worker, with only generated provider infrastructure.
(() => {
  const text=()=>document.body.textContent;
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  async function wait(predicate,stage){const end=Date.now()+15000;while(!predicate()){check(Date.now()<end,'Declared native create wait expired: '+stage+'; '+[...document.querySelectorAll('[role="alert"]')].map(n=>n.textContent).join(' | '));await new Promise(resolve=>setTimeout(resolve,20));}}
  const button=label=>[...document.querySelectorAll('button')].find(n=>n.textContent===label);
  const click=async label=>{await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();};
  function input(id,value){const node=document.getElementById(id);Object.getOwnPropertyDescriptor(node.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(node,value);node.dispatchEvent(new Event('input',{bubbles:true}));}
  (async()=>{
    const config=window.__snapshotFixture;
    await wait(()=>document.getElementById('connection-mode'),'mount');const mode=document.getElementById('connection-mode');mode.value='remote';mode.dispatchEvent(new Event('change',{bubbles:true}));
    await wait(()=>document.getElementById('remote-token'),'remote form');input('remote-endpoint',config.endpoint);input('remote-token',config.token);input('remote-ca',config.ca);
    await new Promise(resolve=>setTimeout(resolve,50));document.querySelector('.remote-connection form').requestSubmit();
    await wait(()=>document.getElementById('create-packages'),'create form');
    check(document.getElementById('create-packages').value===''&&!document.getElementById('create-final-snapshot').checked,'Declared native options enabled by default');
    input('create-public-key',config.publicKey);input('create-packages','build-essential\ngit=1:2.0-1');document.getElementById('create-final-snapshot').click();
    await new Promise(resolve=>setTimeout(resolve,50));await click('Save definition and preview');await wait(()=>button('Create confirmed VM'),'native plan');
    check(text().includes('Declared packages: build-essential, git=1:2.0-1')&&text().includes('Requested · disk-only'),'Native declaration review missing');
    check(!document.getElementById('create-confirm').checked&&button('Create confirmed VM').disabled,'Native creation skipped consent');
    document.getElementById('create-confirm').click();await click('Create confirmed VM');
    await wait(()=>button('Retry same confirmed request')&&!button('Retry same confirmed request').disabled,'lost accepted reply');
    check(!button('Edit definition')&&document.getElementById('create-packages').matches(':disabled'),'Unknown native submission editable');
    await click('Retry same confirmed request');await wait(()=>button('Refresh creation status'),'operation accepted');
    const end=Date.now()+20000;
    while(!text().includes('Creation: succeeded')){
      check(Date.now()<end&&!text().includes('Creation: failed'),'Declared native creation failed or timed out');
      await click('Refresh creation status');await new Promise(resolve=>setTimeout(resolve,100));
    }
    check(text().includes('Guest initialized at 192.0.2.42')&&!button('Retry same confirmed request'),'Missing final readiness or known-operation POST');
    check(!document.querySelector('.remote-ssh-target'),'Readiness automatically selected SSH');
    await click('Read recorded steps');await wait(()=>text().includes('Create final disk-only snapshot: succeeded'),'recorded native stages');check(text().includes('Install declared packages: succeeded')&&text().includes('Saved stage evidence only'),'Native progress omitted packages or historical context');
    await click('Prepare SSH connection');await wait(()=>text().includes('SSH target: debian@192.0.2.42'),'explicit guest handoff');check(text().includes('independently verify its host key'),'SSH trust bypassed');
    await click('Disconnect');await window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:true,failure:null});
  })().catch(error=>window.__TAURI_INTERNALS__.invoke('smoke_report',{ok:false,failure:String(error)}));
})();
