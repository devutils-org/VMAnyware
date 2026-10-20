// Actual Android IPC -> verified HTTPS proxy -> owned production control plane.
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { join, isAbsolute } from "node:path";
import { createServer as httpsServer } from "node:https";
import { request as httpRequest, createServer as httpServer } from "node:http";
import { randomBytes, X509Certificate } from "node:crypto";
const exec = promisify(execFile);
export async function exerciseRemote({command,check,delay,evaluate,invoke,launch,detach,directory,creation=false,created,phrase,nativeView}) {
  const direct = process.env.VMANYWARE_ANDROID_HTTPS_DIRECT === "1";
  const emulatorHost = direct ? "10.0.2.2" : "127.0.0.1";
  const executable = process.env.VMANYWARE_ANDROID_CONTROL_PLANE;
  check(executable && isAbsolute(executable), "Set VMANYWARE_ANDROID_CONTROL_PLANE to the owned production server executable");
  const scratch = await mkdtemp("/tmp/vmanyware-android-https-");
  const token = randomBytes(32).toString("hex"), wrongToken = randomBytes(32).toString("hex");
  let child, proxy, wrongHost, reverse, wrongReverse, provider;
  const sockets = new Set(), requests = [];
  let diagnostics = "";
  async function listen(server) {
    server.on("connection", socket => { sockets.add(socket); socket.on("close",()=>sockets.delete(socket)); });
    await new Promise((resolve,reject)=>{server.once("error",reject);server.listen(0,"127.0.0.1",resolve);});
    return server.address().port;
  }
  async function wait(condition,label) {
    console.log(`Android HTTPS stage: ${label}`);
    const deadline = Date.now()+15000;
    while (!await condition()) { check(Date.now()<deadline,`Android HTTPS stage timed out: ${label}`); await delay(100); }
  }
  const has = text => evaluate(`document.body.innerText.includes(${JSON.stringify(text)})`);
  async function input(id,value) {
    await evaluate(`(()=>{const field=document.getElementById(${JSON.stringify(id)});Object.getOwnPropertyDescriptor(field.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(field,${JSON.stringify(value)});field.dispatchEvent(new Event('input',{bubbles:true}))})()`);
    await delay(50);
  }
  async function remoteMode() {
    await evaluate("(()=>{const mode=document.getElementById('connection-mode');Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set.call(mode,'remote');mode.dispatchEvent(new Event('change',{bubbles:true}))})()");
    await wait(()=>evaluate("Boolean(document.getElementById('remote-endpoint'))"),"remote connection form mounted");
  }
  const click = label => evaluate(`(()=>{const button=[...document.querySelectorAll('.remote-connection button')].find(button=>button.textContent===${JSON.stringify(label)});if(!button||button.disabled)throw new Error('Remote button unavailable');button.click()})()`);
  try {
    await exec("openssl",["req","-x509","-newkey","rsa:2048","-nodes","-keyout",join(scratch,"ca.key"),"-out",join(scratch,"ca.pem"),"-days","1","-subj","/CN=Owned Android HTTPS fixture CA","-addext","basicConstraints=critical,CA:TRUE"]);
    for (const [name,ip] of [["server","127.0.0.1"],["wrong-host","127.0.0.2"]]) {
      await exec("openssl",["req","-newkey","rsa:2048","-nodes","-keyout",join(scratch,`${name}.key`),"-out",join(scratch,`${name}.csr`),"-subj","/CN=Owned Android HTTPS fixture"]);
      await writeFile(join(scratch,`${name}.ext`),`subjectAltName=${name === "server" ? "IP:127.0.0.1,IP:10.0.2.2" : `IP:${ip}`}\nbasicConstraints=critical,CA:FALSE\nextendedKeyUsage=serverAuth\n`);
      await exec("openssl",["x509","-req","-in",join(scratch,`${name}.csr`),"-CA",join(scratch,"ca.pem"),"-CAkey",join(scratch,"ca.key"),"-CAcreateserial","-out",join(scratch,`${name}.pem`),"-days","1","-extfile",join(scratch,`${name}.ext`)]);
    }
    const ca = await readFile(join(scratch,"ca.pem"),"utf8");
    // Reserve an available loopback port; the owned binary then binds it itself.
    const reservation = httpServer(); const servicePort = await listen(reservation);
    await new Promise(resolve=>reservation.close(resolve));
    const database = join(scratch,"service.sqlite");
    const providerEnv = {};
    if (creation) {
      check(created?.publicKey && phrase && process.env.VMANYWARE_ANDROID_SSH_FIXTURE, "Creation mode requires the owned Android identity and SSH fixture");
      const providerSecret=randomBytes(32).toString("hex");
      const {createProvider}=await import("./provider.mjs");
      provider=await createProvider({tls:{key:await readFile(join(scratch,"server.key")),cert:await readFile(join(scratch,"server.pem"))},listen,publicKey:created.publicKey,authorization:`PVEAPIToken=fixture@pve!android=${providerSecret}`,check});
      const catalog=await readFile(new URL("../../examples/templates.yaml",import.meta.url),"utf8");
      await writeFile(join(scratch,"templates.yaml"),catalog+"    imageDefaultUser: {name: debian, templateVersion: debian-13-cloud-20261016, passwordLocked: true, noAuthorizedKeys: true}\n");
      Object.assign(providerEnv,{VMANYWARE_PROVISIONING_ENABLED:"true",VMANYWARE_PROXMOX_URL:`https://127.0.0.1:${provider.port}`,VMANYWARE_PROXMOX_TOKEN_ID:"fixture@pve!android",VMANYWARE_PROXMOX_TOKEN_SECRET:providerSecret,VMANYWARE_PROXMOX_CA:join(scratch,"ca.pem"),VMANYWARE_TEMPLATES:join(scratch,"templates.yaml")});
    }
    child = spawn(executable,[],{env:{PATH:"/usr/bin:/bin",VMANYWARE_PORT:String(servicePort),VMANYWARE_DATABASE:database,VMANYWARE_ACCESS_TOKEN:token,...providerEnv},stdio:["ignore","pipe","pipe"]});
    child.on("error",error=>{diagnostics=error.message;});
    const exited = new Promise(resolve=>{child.once("exit",resolve);child.once("error",resolve);});
    child.stdout.on("data",chunk=>{diagnostics=(diagnostics+chunk).slice(-4096);});
    child.stderr.on("data",chunk=>{diagnostics=(diagnostics+chunk).slice(-4096);});
    child.ownedExit = exited;
    await wait(async()=>{
      check(child.pid && child.exitCode===null,`Owned control plane unavailable: ${diagnostics}`);
      try { return (await fetch(`http://127.0.0.1:${servicePort}/api/v1/health`,{headers:{authorization:`Bearer ${token}`},signal:AbortSignal.timeout(500)})).ok; } catch { return false; }
    },"production service startup");
    // Seed only this freshly created disposable DB. The failed retained operation
    // is a recovery fixture, never evidence that a VM was actually provisioned.
    if (!creation) await exec("python3",["-c",`
import sqlite3,sys
c=sqlite3.connect(sys.argv[1]);c.execute('PRAGMA foreign_keys=ON')
c.execute("INSERT INTO definitions(name,definition_yaml,definition_hash,revision) VALUES(?,?,?,1)",('android-retained','fixture-only','a'*64))
c.execute("INSERT INTO instances(id,definition_name,definition_hash,definition_revision,definition_yaml,provider_scope,node,vmid) VALUES(?,?,?,1,?,?,?,202)",('e'*32,'android-retained','a'*64,'fixture-only','c'*64,'fixture-node'))
c.execute("INSERT INTO operations(id,request_key,kind,instance_id,status,failure_code,approval_json,input_hash,confirmed_at) VALUES(?,?,?,?,?,?,?,?,0)",('d'*32,'android-retained-fixture','vm_provision','e'*32,'failed','provider_not_configured','{}','b'*64))
c.commit();c.close()
`,database]);
    const serve = (request,response) => {
      requests.push({method:request.method,path:request.url});
      const creationWrite = creation && request.method === "POST" && ["/api/v1/definitions","/api/v1/provisioning/plan","/api/v1/provisioning/create"].includes(request.url);
      if (request.method!=="GET" && !creationWrite) {response.writeHead(405);response.end();return;}
      const upstream = httpRequest({host:"127.0.0.1",port:servicePort,path:request.url,method:request.method,headers:{authorization:request.headers.authorization || "",...(request.headers["content-type"] ? {"content-type":request.headers["content-type"]} : {}),...(request.headers["idempotency-key"] ? {"idempotency-key":request.headers["idempotency-key"]} : {})}},reply=>{
        response.writeHead(reply.statusCode,reply.headers);reply.pipe(response);
      });
      upstream.on("error",()=>{response.writeHead(502);response.end();});request.pipe(upstream);
    };
    proxy = httpsServer({key:await readFile(join(scratch,"server.key")),cert:await readFile(join(scratch,"server.pem"))},serve);
    wrongHost = httpsServer({key:await readFile(join(scratch,"wrong-host.key")),cert:await readFile(join(scratch,"wrong-host.pem"))},serve);
    const httpsPort = await listen(proxy), mismatchPort = await listen(wrongHost);
    if (!direct) {
      reverse=`tcp:${httpsPort}`;wrongReverse=`tcp:${mismatchPort}`;
      await command("reverse","--no-rebind",reverse,reverse);
      await command("reverse","--no-rebind",wrongReverse,wrongReverse);
    } else {
      check((await command("reverse","--list")) === "", "Direct mode requires no ADB reverse mappings");
    }
    const endpoint=`https://${emulatorHost}:${httpsPort}`;
    for (const caPem of ["not a certificate", `${ca}${ca}`, "-----BEGIN PRIVATE KEY-----\nfixture\n-----END PRIVATE KEY-----", "A".repeat(16385)]) {
      const invalid = await invoke("connect_remote",{endpoint,token,caPem});
      check(!invalid.ok && invalid.error.includes("public CA"),"Invalid/private/multiple/oversize CA input accepted");
    }
    const absent = await invoke("connect_remote",{endpoint,token});
    check(!absent.ok && absent.error.includes("TLS certificate"),"Untrusted TLS certificate was accepted or crashed native IPC");
    check(requests.length===0,"Untrusted TLS sent HTTP credentials");
    const mismatch = await invoke("connect_remote",{endpoint:`https://${emulatorHost}:${mismatchPort}`,token,caPem:ca});
    check(!mismatch.ok && mismatch.error.includes("TLS certificate"),`Hostname mismatch did not produce the expected TLS refusal: ${JSON.stringify(mismatch)}`);
    check(requests.length===0,"Hostname mismatch sent HTTP credentials");
    await remoteMode();
    await input("remote-endpoint",endpoint);await input("remote-ca",ca);await input("remote-token",wrongToken);
    await click("Connect securely");
    await wait(()=>has("The server rejected the access token."),"invalid token refusal through native HTTPS");
    check(await evaluate("document.getElementById('remote-token').value === ''"),"Rejected token remained in the form");
    async function connect() {
      await input("remote-endpoint",endpoint);await input("remote-ca",ca);await input("remote-token",token);
      await click("Connect securely");await wait(()=>has(`Connected to ${endpoint}`),"verified authenticated native connection");
      check(await evaluate("document.getElementById('remote-token').value === ''"),"Connected token remained in the form");
    }
    await connect();
    if (creation) {
      const {exerciseCreate}=await import("./create.mjs");
      const {exerciseSsh}=await import("./ssh.mjs");
      const result = await exerciseCreate({check,delay,evaluate,click,input,wait,has,created,phrase,provider,requests,directory,directHttps:direct,
        ssh:()=>exerciseSsh({created,phrase,command,check,delay,evaluate,invoke,launch,detach,directory,nativeView})});
      if (direct && process.env.VMANYWARE_ANDROID_SSH_ADB_TUNNEL !== "1") check(await command("reverse", "--list") === "", "Direct journey left an ADB reverse mapping");
      return result;
    }
    await click("Load remote workspace");
    await wait(()=>has("android-retained"),"actual server environment list");
    await wait(()=>has("Cannot load templates. Check the server catalog and access configuration."),"unconfigured catalog produces actionable native error");
    await click("Find recent creations");await wait(()=>has(`Inspect ${"d".repeat(32)}`),"retained operation list");
    await click(`Inspect ${"d".repeat(32)}`);await wait(()=>has("Recovered creation: failed"),"read-only retained operation and target");
    check(await has("Retained target: android-retained"),"Recovery lost the retained association");
    const beforeStop = requests.length;
    await detach();await command("shell","am","force-stop","dev.vmanyware.client");await launch();await remoteMode();
    check(!(await has("Connected to ")),"Process restart restored an authenticated profile");
    check(await evaluate("document.getElementById('remote-token').value === '' && document.getElementById('remote-ca').value === ''"),"Connection credential/trust fields persisted after restart");
    await delay(250);check(requests.length===beforeStop,"Restart replayed a service request without reconnecting");
    await connect();await click("Find recent creations");await wait(()=>has(`Inspect ${"d".repeat(32)}`),"history after explicit reconnect");
    await click(`Inspect ${"d".repeat(32)}`);await wait(()=>has("Recovered creation: failed"),"read-only recovery after process restart");
    check(await evaluate(`!JSON.stringify(Object.entries(localStorage)).includes(${JSON.stringify(token)})`),"Token entered browser storage");
    await evaluate("document.querySelector('.remote-recovery').scrollIntoView({block:'start'})");
    await wait(()=>evaluate("document.querySelector('.remote-recovery').getBoundingClientRect().top < 100"),"retained-operation screen scrolled into view");
    await delay(500);
    const screenshot = await exec(process.env.VMANYWARE_ANDROID_ADB,["-P","5047","-s","emulator-5580","exec-out","screencap","-p"],{encoding:"buffer",timeout:15000,maxBuffer:8*1024*1024});
    await writeFile(join(directory,"remote-recovery.png"),screenshot.stdout);
    await click("Disconnect");await wait(()=>evaluate("!document.getElementById('remote-endpoint').disabled"),"explicit native disconnect");
    check(await evaluate("document.getElementById('remote-ca').value === ''"),"Disconnect retained explicit CA input");
    check(requests.every(request=>request.method==="GET"),"Recovery submitted a write");
    const report={platform:"android",transport:direct ? "direct emulator host alias with verified HTTPS; no ADB reverse" : "explicit owned ADB reverse tunnel with verified HTTPS",service:"production control-plane binary with disposable database and no Proxmox configuration",caFingerprint:new X509Certificate(ca).fingerprint256,requests,checks:["untrusted issuer and hostname mismatch rejected before HTTP", "invalid token rejected by actual server", "verified native HTTPS and explicit per-connection CA", "actual retained environment/operation reads and actionable missing-catalog error", "restart forgets token/profile/CA with no request replay", "explicit reconnect recovers retained failed operation using GET only", "disconnect and no browser token storage"],limitations:[direct ? "Emulator host-alias networking, not a deployed service or physical-device network" : "ADB tunnel, not direct emulator/deployed networking", "Seeded failed operation, not real provisioning/readiness evidence", "Debug emulator, not physical-device or release support"]};
    await writeFile(join(directory,"remote-report.json"),JSON.stringify(report,null,2)+"\n");
    console.log(`Android HTTPS passed: ${report.checks.join(", ")}`);
    return report;
  } finally {
    if(reverse)await command("reverse","--remove",reverse).catch(()=>{});
    if(wrongReverse)await command("reverse","--remove",wrongReverse).catch(()=>{});
    for(const socket of sockets)socket.destroy();
    for(const server of [proxy,wrongHost,provider?.server])if(server)await new Promise(resolve=>server.close(resolve));
    if(child && child.exitCode===null) {
      child.kill("SIGTERM");let timer;
      await Promise.race([child.ownedExit,new Promise(resolve=>{timer=setTimeout(()=>{child.kill("SIGKILL");resolve();},7000);})]);clearTimeout(timer);
      await child.ownedExit;
    }
    await rm(scratch,{recursive:true,force:true});
  }
}
