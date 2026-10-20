// UI-only snapshot experiment; no real server, native storage or VM access.
(() => {
  const id="a".repeat(32), scope="b".repeat(64), calls=[];
  const environment={id,definitionName:"snapshot-lab",providerReference:{scopeId:scope,node:"fixture-node",vmid:202},status:"registered"};
  const report=snapshots=>({instanceId:id,providerScopeId:scope,node:"fixture-node",vmid:202,snapshots});
  let reads=0, release;
  window.isTauri=true;
  window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
    calls.push({command,args});
    if(command.endsWith("_support"))return false;
    if(command==="connect_remote")return {connectionId:1,endpoint:"https://fixture.example.test/"};
    if(command==="disconnect_remote" || command==="cancel_ssh_host")return;
    if(command==="read_remote_images")return [];
    if(command==="read_remote_environments")return [environment];
    if(command==="read_remote_snapshots"){
      check(args.connectionId===1 && args.instanceId===id,"Wrong snapshot target/connection");
      reads++;
      if(reads===1)return report([]);
      if(reads===2)return report([{name:"before-work",description:"<b>literal description</b>",createdAt:null,includesRam:null,parent:"baseline",providerState:"prepare"}]);
      if(reads===3)return {...report([]),vmid:999};
      if(reads===4)throw "Cannot load snapshots. Check target, provider permissions and server availability.";
      return new Promise(resolve=>{release=resolve;});
    }
    throw `Unexpected snapshot fixture command: ${command}`;
  }};
  const text=()=>document.body?.textContent || "";
  function check(ok,message){if(!ok)throw new Error(message);}
  async function wait(predicate){const end=Date.now()+5000;while(!predicate()){check(Date.now()<end,`Snapshot wait timed out: ${text().slice(-500)}`);await new Promise(resolve=>setTimeout(resolve,10));}}
  const button=label=>[...document.querySelectorAll("button")].find(element=>element.textContent===label);
  function input(id,value){const element=document.getElementById(id);Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value").set.call(element,value);element.dispatchEvent(new Event("input",{bubbles:true}));}
  (async()=>{
    await wait(()=>document.getElementById("connection-mode"));
    const mode=document.getElementById("connection-mode");mode.value="remote";mode.dispatchEvent(new Event("change",{bubbles:true}));
    await wait(()=>document.getElementById("remote-token"));
    input("remote-endpoint","https://fixture.example.test");input("remote-token","fixture-only-access-token-0123456789abcdef");
    await new Promise(resolve=>setTimeout(resolve,20));
    document.querySelector(".remote-connection form").requestSubmit();
    await wait(()=>button("Load remote workspace") && !button("Load remote workspace").disabled);button("Load remote workspace").click();
    await wait(()=>button("List snapshots for VM 202"));
    check(reads===0,"Snapshot read happened without explicit request");
    button("List snapshots for VM 202").click();
    await wait(()=>text().includes("No snapshots reported for this VM"));
    check(text().includes("Storage support has not been checked"),"Empty list claimed storage support");
    button("List snapshots for VM 202").click();
    await wait(()=>text().includes("before-work"));
    check(text().includes("snapshot-lab · fixture-node · VM 202"),"Snapshot target is unclear");
    check(text().includes("not independent backups") && text().includes("may be incomplete"),"Backup/incomplete distinctions missing");
    check(text().includes("Created: Unknown · RAM: Unknown") && text().includes("Parent: baseline"),"Unknown metadata or parent lost");
    check(text().includes("<b>literal description</b>") && !document.querySelector(".remote-snapshots b"),"Provider description rendered as HTML");
    button("List snapshots for VM 202").click();
    await wait(()=>text().includes("belongs to a different target"));
    check(!text().includes("before-work"),"Mismatched response retained prior snapshot results");
    button("List snapshots for VM 202").click();
    await wait(()=>text().includes("Cannot load snapshots"));
    check(!text().includes("No snapshots reported"),"Provider failure became an empty success");
    button("List snapshots for VM 202").click();await wait(()=>release);
    check(button("List snapshots for VM 202").disabled,"Concurrent snapshot read allowed");
    button("Disconnect").click();await wait(()=>!text().includes("Connected to"));
    release(report([{name:"STALE_SNAPSHOT",description:"",createdAt:null,includesRam:false,parent:null,providerState:null}]));
    await new Promise(resolve=>setTimeout(resolve,50));
    check(!text().includes("STALE_SNAPSHOT"),"Snapshot response survived disconnect");
    check(calls.every(call=>!commandIsWrite(call.command)),"Snapshot UI submitted a mutation");
    return {ok:true,checks:["explicit scoped snapshot read","empty and provider error states","target mismatch refusal","unknown/incomplete metadata","escaped descriptions and backup distinction","stale disconnect result suppressed","no mutation"]};
  })().catch(error=>({ok:false,error:error.message+"; commands="+calls.map(call=>call.command).join(",")+"; snapshotReads="+reads})).then(result=>fetch("/__instance_fixture/result",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify(result)}));
  function commandIsWrite(command){return /create|restore|shutdown|start_ssh|lifecycle/.test(command);}
})();
