// Opt-in real Android/WebView/Tauri smoke. Never accesses a real device or service.
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { createHash, randomUUID } from "node:crypto";
import { createServer as tcpServer } from "node:net";
const exec = promisify(execFile);
const adb = process.env.VMANYWARE_ANDROID_ADB;
const serial = "emulator-5580", port = "5047", app = "dev.vmanyware.client";
const args = process.argv.slice(2), positional = [];
let apk;
const flags = new Set(["--reset-owned-app", "--remote", "--create", "--ssh", "--keystore"]);
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--apk") { if (apk || !args[i+1] || args[i+1].startsWith("--")) throw Error("Supply one APK path"); apk = resolve(args[++i]); }
  else if (args[i].startsWith("--")) { if (!flags.has(args[i])) throw Error("Unknown Android fixture option"); }
  else positional.push(args[i]);
}
if (positional.length > 1) throw Error("Supply only one fixture output directory");
const directory = resolve(positional[0] || ".lab/android-native");
const keystore = args.includes("--keystore");
if (keystore !== Boolean(apk)) throw Error("--keystore requires --apk; --apk is only accepted for the Keystore journey");
if (!adb || !process.argv.includes("--reset-owned-app")) throw new Error("Set VMANYWARE_ANDROID_ADB and opt in with --reset-owned-app; only the owned vmanyware-native-smoke AVD is allowed");
if (process.argv.includes("--create") && (!process.argv.includes("--remote") || process.argv.includes("--ssh"))) throw new Error("--create requires --remote and already includes its SSH journey; omit --ssh");
await mkdir(directory, { recursive: true });
const command = async (...args) => (await exec(adb, ["-P", port, "-s", serial, ...args], { timeout: 15000, maxBuffer: 1024 * 1024 })).stdout.trim();
const check = (value, message) => { if (!value) throw new Error(message); };
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
check((await command("emu", "avd", "name")).split(/\r?\n/)[0] === "vmanyware-native-smoke", "Wrong AVD; refusing app changes");
check(await command("shell", "getprop", "ro.kernel.qemu") === "1", "Refusing a physical device");
check(await command("shell", "getprop", "sys.boot_completed") === "1", "Owned emulator is not booted");
let apkSha256;
if (keystore) {
  apkSha256 = createHash("sha256").update(await readFile(apk)).digest("hex");
  const path = (await command("shell", "pm", "path", app)).match(/^package:(\/data\/app\/[A-Za-z0-9_./=+~\-]+\/base\.apk)$/)?.[1];
  check(path, "Keystore journey requires one installed standalone APK");
  check((await command("shell", "toybox", "sha256sum", path)).split(/\s+/)[0] === apkSha256, "Installed APK differs from the reviewed local artifact; refusing reset");
}
// Boot completion does not establish a usable Android network. A bounded owned
// TCP probe gates direct mode independently of application/TLS error messages.
let networkReport;
if (process.env.VMANYWARE_ANDROID_HTTPS_DIRECT === "1" || (process.argv.includes("--ssh") && process.env.VMANYWARE_ANDROID_SSH_ADB_TUNNEL !== "1")) {
  console.log("Android native stage: direct host-alias TCP readiness");
  check(await command("reverse", "--list") === "", "Direct mode requires no ADB reverse mappings");
  const marker = `VMANYWARE_OWNED_TCP_${randomUUID()}`, connections = new Set();
  const probe = tcpServer(connection => {
    connections.add(connection); connection.on("close", () => connections.delete(connection));
    connection.end(`${marker}\n`);
  });
  const started = Date.now(); let attempts = 0, ready = false;
  try {
    await new Promise((resolve, reject) => { probe.once("error", reject); probe.listen(0, "127.0.0.1", resolve); });
    while (Date.now() - started < 30000) {
      attempts++;
      try { ready = await command("shell", "toybox", "nc", "-w", "2", "10.0.2.2", String(probe.address().port)) === marker; } catch { /* Route setup may still be in flight. */ }
      if (ready) break;
      await delay(250);
    }
    check(ready, "Owned host-alias TCP route did not become usable; no tunnel fallback was selected");
    networkReport = {target:"10.0.2.2 -> owned loopback listener",attempts,elapsedMs:Date.now()-started,adbReverseMappings:0};
  } finally {
    for (const connection of connections) connection.destroy();
    await new Promise(resolve => probe.close(resolve));
  }
}
let socket, forward, next = 0;
const waiting = new Map();
async function detach() {
    const previous = socket; socket = null;
  if (previous && previous.readyState !== WebSocket.CLOSED) {
    await new Promise(resolve => {
      const timer = setTimeout(resolve, 1000);
      previous.addEventListener("close", () => { clearTimeout(timer); resolve(); }, {once:true});
      previous.close();
    });
  }
  if (forward) { await command("forward", "--remove", `tcp:${forward}`).catch(() => {}); forward = null; }
}
async function attach() {
  await detach();
  let target;
  for (let attempt = 0; attempt < 60; attempt++) {
    try {
      const pid = await command("shell", "pidof", app);
      check(/^\d+$/.test(pid), "App process unavailable");
      if (!forward) forward = await command("forward", "tcp:0", `localabstract:webview_devtools_remote_${pid}`);
      const pages = await (await fetch(`http://127.0.0.1:${forward}/json/list`)).json();
      target = pages.find(page => page.type === "page" && page.url === "http://tauri.localhost/");
      if (target) break;
    } catch { /* The WebView debug socket appears after native startup. */ }
    await delay(250);
  }
  check(target, "No owned app WebView became available");
  socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.addEventListener("open", resolve, {once:true}); socket.addEventListener("error", reject, {once:true}); });
  socket.addEventListener("message", event => {
    const result = JSON.parse(event.data);
    if (result.id && waiting.has(result.id)) {
      const {resolve, reject, timer} = waiting.get(result.id); waiting.delete(result.id); clearTimeout(timer);
      result.error ? reject(new Error(result.error.message)) : resolve(result.result);
    }
  });
  socket.addEventListener("close", () => {
    for (const task of waiting.values()) { clearTimeout(task.timer); task.reject(new Error("Owned WebView closed")); }
    waiting.clear();
  });
}
async function evaluate(expression) {
  const id = ++next;
  const response = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(new Error("Owned WebView command timed out")); }, 15000);
    waiting.set(id, {resolve, reject, timer});
    socket.send(JSON.stringify({id,method:"Runtime.evaluate",params:{expression,awaitPromise:true,returnByValue:true}}));
  });
  if (response.exceptionDetails) throw new Error("Owned WebView evaluation failed");
  return response.result.value;
}
async function rendered() {
  for (let n = 0; n < 40; n++) {
    if (await evaluate("Boolean(document.readyState === 'complete' && document.querySelector('#ssh-host') && window.__TAURI_INTERNALS__)")) {
      // Android publishes its debug socket before the startup navigation and
      // native WebView handoff have settled. Do not bind an IPC promise there.
      await delay(500);
      if (await evaluate("document.readyState === 'complete' && location.origin === 'http://tauri.localhost'")) return;
    }
    await delay(250);
  }
  throw new Error("Application screens did not mount");
}
async function launch() {
  await command("shell", "am", "start", "-n", `${app}/.MainActivity`);
  await attach(); await rendered();
}
const phrase = `Disposable Android fixture ${randomUUID()}`;
// The helper invokes the actual Tauri bridge; there is no simulated IPC here.
async function invoke(name, args) {
  return evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(name)}, ${JSON.stringify(args || {})}).then(value => ({ok:true,value}), error => ({ok:false,error:typeof error === "string" ? error : "native rejection"}))`);
}
try {
  check(await command("shell", "pm", "clear", app) === "Success", "Cannot reset owned fixture app");
  console.log("Android native stage: initial launch");
  await launch();
  await evaluate("[...document.querySelectorAll('button')].find(button => button.textContent === 'Check native bridge').click()");
  for (let n=0; n<40; n++) {
    if (await evaluate("document.body.textContent.includes('Native bridge connected (android)')")) break;
    await delay(100);
  }
  check(await evaluate("document.body.textContent.includes('Native bridge connected (android)')"), "Native platform bridge did not report Android");
  check((await invoke("identity_support")).value === true, "Application key backend missing");
  check((await invoke("ssh_host_support")).value === true, "SSH host backend missing");
  check((await invoke("ssh_agent_support")).value === false, "macOS agent incorrectly offered");
  console.log("Android native stage: encrypted identity");
  const created = await invoke("create_app_identity", {passphrase:phrase});
  check(created.ok && created.value.publicKey.startsWith("ssh-ed25519 "), "Native encrypted identity creation failed");
  check(!(await invoke("unlock_app_identity", {passphrase:`Incorrect ${randomUUID()}`})).ok, "Wrong passphrase accepted");
  check(!(await invoke("create_app_identity", {passphrase:phrase})).ok, "Existing identity overwritten");
  check((await invoke("connect_remote", {endpoint:"http://127.0.0.1",token:"disposable-fixture-token-0123456789abcdef"})).error?.includes("HTTPS"), "HTTPS guard missing from native remote command");
  check((await invoke("inspect_ssh_host", {host:"https://invalid.example/",port:22})).error === "Invalid SSH endpoint", "Native SSH host endpoint guard missing");
  check((await invoke("list_app_ssh_identity", {host:"https://invalid.example/",port:22,username:"fixture",passphrase:phrase})).error === "Invalid SSH endpoint", "Native SSH identity endpoint guard missing");
  check((await invoke("write_ssh_session", {sessionId:"not-a-session",bytes:[3]})).error === "SSH session unavailable or closed", "Native SSH session ownership guard missing");
  if (keystore) {
    console.log("Android native stage: scoped Keystore save/read");
    const {prepareKeystore} = await import("./keystore.mjs");
    await prepareKeystore({invoke,check,created:created.value,phrase});
  }
  console.log("Android native stage: process restart");
  await detach();
  await command("shell", "am", "force-stop", app);
  await launch();
  const reopened = await invoke("unlock_app_identity", {passphrase:phrase});
  check(reopened.ok && reopened.value.fingerprint === created.value.fingerprint && reopened.value.publicKey === created.value.publicKey, "Android identity did not survive process restart");
  let keystoreReport;
  if (keystore) {
    const {finishKeystore} = await import("./keystore.mjs");
    keystoreReport = await finishKeystore({invoke,check,created:created.value,phrase});
  }
  let remoteReport;
  if (process.argv.includes("--remote")) {
    console.log("Android native stage: authenticated HTTPS journey");
    const { exerciseRemote } = await import("./remote.mjs");
    remoteReport = await exerciseRemote({command,check,delay,evaluate,invoke,launch,detach,directory,creation:process.argv.includes("--create"),created:created.value,phrase,
      nativeView: async () => JSON.parse((await (await fetch(`http://127.0.0.1:${forward}/json/list`)).json()).find(page => page.type === "page").description)});
  }
  let sshReport;
  if (process.argv.includes("--ssh")) {
    console.log("Android native stage: SSH journey");
    const { exerciseSsh } = await import("./ssh.mjs");
    sshReport = await exerciseSsh({created:created.value,phrase,command,check,delay,evaluate,invoke,launch,detach,directory,
      nativeView: async () => JSON.parse((await (await fetch(`http://127.0.0.1:${forward}/json/list`)).json()).find(page => page.type === "page").description)});
  }
  const layout = await evaluate("({width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth})");
  check(layout.scrollWidth <= layout.width + 1, "Android launch screen overflows horizontally");
  await evaluate("scrollTo(0,0)");
  // DOM readiness alone previously let an unpainted white native surface pass.
  // Inspect the actual device framebuffer before recording visual evidence.
  let painted = false;
  for (let attempt = 0; attempt < 12 && !painted; attempt++) {
    const {stdout: frame} = await exec(adb,["-P",port,"-s",serial,"exec-out","screencap"],{encoding:"buffer",maxBuffer:16*1024*1024,timeout:15000});
    const width = frame.readUInt32LE(0), height = frame.readUInt32LE(4), format = frame.readUInt32LE(8);
    const header = frame.length - width * height * 4;
    check([1,2].includes(format) && [12,16].includes(header), "Unsupported device framebuffer format");
    let dark = 0, light = 0, samples = 0;
    for (let y = Math.ceil(height * .09); y < height * .30; y += 8) {
      for (let x = Math.ceil(width * .05); x < width * .95; x += 8) {
        const at = header + (y * width + x) * 4;
        const [r,g,b] = frame.subarray(at, at+3); samples++;
        if (r < 80 && g < 80 && b < 80) dark++;
        if (r > 160 && g > 160 && b > 160) light++;
      }
    }
    painted = dark > samples / 2 && light > 20;
    if (!painted) await delay(500);
  }
  check(painted, "Android native surface remained blank despite a mounted DOM");
  const screenshot = await exec(adb,["-P",port,"-s",serial,"exec-out","screencap","-p"],{encoding:"buffer",maxBuffer:8*1024*1024,timeout:15000});
  await writeFile(join(directory,"launch.png"),screenshot.stdout);
  const report = {platform:"android",apkSha256,keystore:keystoreReport,release:await command("shell","getprop","ro.build.version.release"),api:await command("shell","getprop","ro.build.version.sdk"),abi:await command("shell","getprop","ro.product.cpu.abi"),network:networkReport,ssh:sshReport,remote:remoteReport,layout,fingerprint:created.value.fingerprint,checks:["actual native platform/SSH command dispatch", "encrypted identity create/wrong phrase/no replacement", "same identity after process restart", "invalid remote/SSH inputs rejected", "unowned SSH session rejected", "painted native surface, mobile layout and screenshot"],limitations:[remoteReport ? "No real VM creation, direct deployed networking, physical device or full background lifecycle journey" : sshReport ? "No authenticated remote control plane, VM creation, physical keyboard/device or background lifecycle journey in this smoke" : "No authenticated remote server, VM creation, SSH connection or keyboard/lifecycle journey in this smoke", "Emulator debug build, not physical-device or release evidence"]};
  await writeFile(join(directory,"report.json"),JSON.stringify(report,null,2)+"\n");
  console.log(`Android native smoke passed: ${report.checks.join(", ")}`);
} finally {
  await detach();
  await command("shell","am","force-stop",app).catch(() => {});
}
