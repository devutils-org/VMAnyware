// In-memory Proxmox protocol peer. Never connects to or changes a real VM.
import { createServer } from "node:https";
export async function createProvider({tls,listen,publicKey,authorization,check}) {
  const requests=[], errors=[];
  const cloneTask="UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:";
  const startTask="UPID:pve-test:00000001:00000002:00000003:qmstart:202:fixture@pve:";
  const target={name:"mobile-lab",template:0,digest:"d".repeat(40),scsi0:"local-lvm:vm-202-disk-0,size=8G",ide2:"local-lvm:vm-202-cloudinit,media=cdrom,size=4M",net0:"virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1"};
  let running=false, regenerated=false;
  const server=createServer(tls,async(request,response)=>{
    try {
      check(request.headers.authorization===authorization,"Provider received wrong fixture authorization");
      const chunks=[];let length=0;
      for await(const chunk of request){length+=chunk.length;check(length<=16384,"Provider body too large");chunks.push(chunk);}
      const body=Buffer.concat(chunks).toString(),form=Object.fromEntries(new URLSearchParams(body));
      const method=request.method,path=request.url;
      requests.push({method,path});
      let data;
      if(method==="GET" && path.endsWith("/version"))data={version:"9.2.2",release:"fixture"};
      else if(method==="GET" && path.endsWith("/nodes"))data=[{node:"pve-test",status:"online"}];
      else if(method==="GET" && path.endsWith("/qemu"))data=[{vmid:9000,name:"fixture-template",status:"stopped",template:1}];
      else if(method==="GET" && (path.endsWith("/qemu/9000/config")||path.endsWith("/qemu/9000/config?current=1")))data={template:1,digest:"c".repeat(40),scsi0:"local-lvm:base-9000-disk-0,size=8G",ide2:"local-lvm:vm-9000-cloudinit,media=cdrom"};
      else if(method==="GET" && path.endsWith("/storage"))data=[{storage:"local-lvm",enabled:1,active:1,content:"images",avail:1099511627776}];
      else if(method==="GET" && path.endsWith("/network"))data=[{iface:"vmbr0",active:1,type:"bridge"}];
      else if(method==="GET" && path.includes("/cluster/nextid"))data=202;
      else if(method==="GET" && path.endsWith("/pending"))data=[];
      else if(method==="GET" && path.endsWith("/qemu/9000/status/current"))data={vmid:9000,status:"stopped"};
      else if(method==="POST" && path.endsWith("/qemu/9000/clone")){
        check(form.newid==="202" && form.name==="mobile-lab" && form.full==="1","Wrong full-clone target");
        check(!target.description,"Duplicate clone mutation");target.description=form.description;data=cloneTask;
      }
      else if(method==="GET" && path.includes("/tasks/"))data={node:"pve-test",upid:path.includes("qmclone")?cloneTask:startTask,status:"stopped",exitstatus:"OK"};
      else if(method==="GET" && (path.endsWith("/qemu/202/config")||path.endsWith("/qemu/202/config?current=1")))data=target;
      else if(method==="PUT" && path.endsWith("/qemu/202/config")){
        check(!running,"Configured a running target");
        if("ciupgrade" in form){
          check(form.ciupgrade === "0" && form.citype === "nocloud" && form.ipconfig0 === "ip=dhcp", "Wrong Cloud-Init settings");
          check(form.net0.includes("bridge=vmbr0") && !form.ciuser, "Wrong guest network or user override");
          check(decodeURIComponent(form.sshkeys).trim()===publicKey.trim(),"Cloud-Init key differs from Android identity");
          for(const key of ["net0","ipconfig0","citype","sshkeys"])target[key]=form[key];
          target.ciupgrade=0;delete target.ciuser;target.digest="f".repeat(40);
        }else{
          target.tags=form.tags;
          check(target.tags===`vmanyware-${target.description.split("\n")[0].split(" ").at(-1)}`,"Missing or wrong created-instance tag");
          for(const key of ["sockets","cores","vcpus","cpulimit","memory","balloon"])target[key]=Number(form[key]);
          target.digest="e".repeat(40);
        }
        data=null;
      }
      else if(method==="GET" && path.endsWith("/qemu/202/status/current"))data={vmid:202,status:running?"running":"stopped",qmpstatus:running?"running":"stopped"};
      else if(method==="GET" && path.endsWith("/cloudinit/dump?type=user"))data=`#cloud-config\nhostname: mobile-lab\nmanage_etc_hosts: true\nfqdn: mobile-lab\nusers: [default]\nssh_authorized_keys:\n  - ${publicKey.trim()}\nchpasswd:\n  expire: False\n`;
      else if(method==="GET" && path.endsWith("/cloudinit/dump?type=network"))data="version: 1\nconfig:\n  - type: physical\n    name: eth0\n    mac_address: 'bc:24:11:12:34:56'\n    subnets:\n      - type: dhcp4\n  - type: nameserver\n    address: [192.0.2.53]\n    search: [example.test]\n";
      else if(method==="PUT" && path.endsWith("/qemu/202/cloudinit")){check(!regenerated,"Duplicate seed regeneration");regenerated=true;data=null;}
      else if(method==="POST" && path.endsWith("/qemu/202/status/start")){check(regenerated&&!running,"Start before seed or duplicate start");running=true;data=startTask;}
      else if(method==="GET" && path.endsWith("/agent/network-get-interfaces"))data={result:[{name:"ens18","hardware-address":"bc:24:11:12:34:56","ip-addresses":[{"ip-address-type":"ipv4","ip-address":"192.0.2.42",prefix:24}]}]};
      else if(method==="POST" && path.endsWith("/agent/exec")){
        check(running && regenerated, "Guest probe before completed seed/start");
        check(JSON.stringify(JSON.parse(body))===JSON.stringify({command:["/usr/bin/cloud-init","status","--format","json"]}),"Unexpected guest command");data={pid:42};
      }
      else if(method==="GET" && path.endsWith("/agent/exec-status?pid=42")){
        const stage={errors:[],recoverable_errors:{},start:10,finished:11};
        data={exited:1,exitcode:0,"out-data":JSON.stringify({status:"done",extended_status:"done",errors:[],recoverable_errors:{},"init-local":stage,init:stage,"modules-config":stage,"modules-final":stage})};
      }else throw new Error(`Unexpected provider request: ${method} ${path}`);
      response.writeHead(200,{"content-type":"application/json"});response.end(JSON.stringify({data}));
    }catch(error){errors.push(error.message);response.writeHead(500);response.end(JSON.stringify({error:"owned_fixture_assertion"}));}
  });
  const port=await listen(server);
  return {server,port,requests,errors,assertComplete(){
    check(errors.length===0,`Provider fixture assertions: ${errors.join(", ")}`);
    for(const [method,suffix] of [["POST","/qemu/9000/clone"],["PUT","/qemu/202/cloudinit"],["POST","/qemu/202/status/start"],["POST","/agent/exec"]])check(requests.filter(r=>r.method===method&&r.path.endsWith(suffix)).length===1,`Missing or duplicate provider stage: ${suffix}`);
    check(requests.filter(r=>r.method==="PUT"&&r.path.endsWith("/qemu/202/config")).length===2,"Wrong configuration write count");
    check(target.cores===2&&target.memory===2048&&target.balloon===0&&running&&regenerated,"Requested resources or final provider state differ");
  }};
}
