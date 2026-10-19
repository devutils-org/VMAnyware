// Real packaged Android -> native SSH -> generated localhost server -> /bin/sh.
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { join } from "node:path";
import { writeFile } from "node:fs/promises";
const exec = promisify(execFile);
const quote = value => `'${value.replaceAll("'", "'\\''")}'`;
export async function exerciseSsh({created,phrase,command,check,delay,evaluate,invoke,launch,detach,directory,nativeView}) {
  const executable = process.env.VMANYWARE_ANDROID_SSH_FIXTURE;
  check(executable, "Set VMANYWARE_ANDROID_SSH_FIXTURE to the owned android_ssh_fixture executable");
  const child = spawn(executable,["--owned-android-fixture"],{detached:true,env:{PATH:"/usr/bin:/bin",...(process.env.LD_LIBRARY_PATH ? {LD_LIBRARY_PATH:process.env.LD_LIBRARY_PATH} : {})},stdio:["pipe","pipe","pipe"]});
  let diagnostics = "", buffered = "";
  const queued = [], readers = [];
  const exited = new Promise(resolve => child.once("exit", resolve));
  child.stderr.on("data", chunk => { diagnostics = (diagnostics + chunk).slice(-8192); });
  const rejectReaders = failure => readers.splice(0).forEach(reader => { clearTimeout(reader.timer); reader.reject(failure); });
  child.on("error", rejectReaders);
  child.on("exit", () => rejectReaders(new Error("Owned SSH fixture exited")));
  child.stdout.on("data", chunk => {
    buffered += chunk;
    while (buffered.includes("\n")) {
      const at = buffered.indexOf("\n"), line = buffered.slice(0, at); buffered = buffered.slice(at+1);
      const value = JSON.parse(line);
      if (readers.length) { const reader = readers.shift(); clearTimeout(reader.timer); reader.resolve(value); }
      else queued.push(value);
    }
  });
  function next() {
    if (queued.length) return Promise.resolve(queued.shift());
    return new Promise((resolve,reject) => {
      const reader = {resolve,reject};
      reader.timer = setTimeout(() => { const at=readers.indexOf(reader); if(at>=0)readers.splice(at,1); reject(new Error("Owned SSH fixture response timed out")); },10000);
      readers.push(reader);
    });
  }
  function request(action) { child.stdin.write(JSON.stringify({action})+"\n"); return next(); }
  async function wait(condition, label, milliseconds = 10000) {
    console.log(`Android SSH stage: ${label}`);
    const deadline=Date.now()+milliseconds;
    while (!await condition()) { check(Date.now()<deadline, `Android SSH stage timed out: ${label}`); await delay(100); }
  }
  const has = value => evaluate(`document.body.textContent.includes(${JSON.stringify(value)})`);
  const click = label => evaluate(`(()=>{const button=[...document.querySelectorAll('button')].find(button=>button.textContent===${JSON.stringify(label)});if(!button||button.disabled)throw new Error('Button unavailable');button.click()})()`);
  async function input(id, value) {
    await evaluate(`(()=>{const input=document.getElementById(${JSON.stringify(id)});Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,${JSON.stringify(value)});input.dispatchEvent(new Event('input',{bubbles:true}))})()`);
    await delay(50);
  }
  const terminalText = () => evaluate("[...document.querySelectorAll('.xterm-rows > div')].map(row=>row.textContent).join('\\n')");
  async function shellLine(text) {
    // Android's input command interprets %s as spaces. The caller only uses
    // fixed disposable shell commands; quote for Android's shell separately.
    await command("shell","input","text",quote(text.replaceAll(" ","%s")));
    await command("shell","input","keyevent","KEYCODE_ENTER");
  }
  let metadata;
  try {
    child.stdin.write(JSON.stringify({publicKey:created.publicKey})+"\n");
    metadata=await next(); check(metadata.ready && metadata.host==="10.0.2.2", "Unexpected SSH fixture target");
    async function inspect() {
      await input("ssh-host",metadata.host); await input("ssh-port",String(metadata.port));
      await click("Inspect host key");
    }
    await inspect();
    await wait(()=>has(metadata.fingerprint),"unknown host fingerprint");
    check(await has("This endpoint has no trusted key"),"A fresh host was already trusted");
    const refused=await invoke("list_app_ssh_identity",{host:metadata.host,port:metadata.port,username:"fixture",passphrase:phrase});
    check(!refused.ok && refused.error.includes("host verification"),"Untrusted host allowed private-key selection");
    check((await request("state")).authAttempts===0,"Untrusted host attempted authentication");
    await evaluate("document.getElementById('ssh-trust-heading').closest('section').querySelector('input[type=checkbox]').click()");
    await click("Trust this host key"); await wait(()=>has("Saved host-key trust"),"explicit trust confirmation");
    await inspect(); await wait(()=>evaluate("Boolean(document.getElementById('ssh-username'))"),"fresh trusted handshake");
    async function select(password = phrase) {
      await input("ssh-username","fixture");
      await evaluate("(()=>{const checkbox=document.getElementById('ssh-username').closest('form').querySelector('input[type=checkbox]');if(!checkbox.checked)checkbox.click()})()");
      await input("ssh-key-passphrase",password);
      await click("Unlock connection identity");
    }
    await select("Incorrect Android fixture passphrase");
    await wait(()=>has("Cannot unlock application SSH identity"),"wrong passphrase refusal");
    check((await request("state")).authAttempts===0,"Wrong passphrase attempted authentication");
    async function choose() {
      await wait(()=>evaluate("Boolean(document.querySelector('input[name=ssh-identity]'))"),"public identity selection");
      check(await has(created.fingerprint),"Wrong public identity displayed");
      await evaluate("document.querySelector('input[name=ssh-identity]').click()");
      await delay(50);
    }
    await select(); await choose(); await click("Check selected identity");
    await wait(()=>has("Authenticated as fixture"),"authentication-only check");
    let state=await request("state");check(state.authAttempts===1 && state.channels===0,"Authentication check opened a channel or tried another key");
    async function open() {
      await select(); await choose(); await click("Open terminal");
      await wait(()=>evaluate("Boolean(document.querySelector('.xterm-helper-textarea'))"),"native terminal mount");
      await wait(async()=> (await terminalText()).includes("fixture> "),"actual shell prompt");
    }
    await open();
    await evaluate("document.querySelector('.mobile-terminal-controls input').scrollIntoView({block:'center'})");
    await delay(250);
    const point=await evaluate("(()=>{const box=document.querySelector('.mobile-terminal-controls input').getBoundingClientRect();return {x:box.left+box.width/2,y:box.top+box.height/2,ratio:devicePixelRatio}})()");
    const view=await nativeView();
    await command("shell","settings","put","secure","show_ime_with_hard_keyboard","1");
    await command("shell","input","tap",String(Math.round(view.screenX+point.x*point.ratio)),String(Math.round(view.screenY+point.y*point.ratio)));
    await wait(()=>evaluate("document.activeElement?.matches('.mobile-terminal-controls input')"),"Android touch terminal focus");
    await wait(async()=> /mInputShown=true|mIsInputViewShown=true/.test(await command("shell","dumpsys","input_method")),"visible Android IME");
    await delay(350);
    check(await evaluate("document.activeElement?.matches('.mobile-terminal-controls input')"),"IME appearance moved focus away from the terminal");
    await shellLine('echo ANDROID_"SSH_OK"');
    await wait(async()=> (await terminalText()).includes("ANDROID_SSH_OK"),"Android key events through PTY to real shell");
    const initial=await request("state");
    check(initial.authAttempts===2 && initial.channels===1 && initial.inputBytes>0,"Terminal did not use one authenticated channel");
    await command("shell","input","keyevent","KEYCODE_BACK");
    await command("shell","settings","put","system","accelerometer_rotation","0");
    await command("shell","settings","put","system","user_rotation","1");
    await wait(async()=> (await request("state")).sizes.some(size=>!initial.sizes.some(old=>old[0]===size[0] && old[1]===size[1])),"rotation-driven native PTY resize");
    const rotatedSize = (await request("state")).sizes.at(-1);
    await evaluate("document.querySelector('.mobile-terminal-controls input').focus()");
    await shellLine("stty size");
    await wait(async()=> (await terminalText()).includes(`${rotatedSize[1]} ${rotatedSize[0]}`),"actual OS PTY size after rotation");
    await command("shell","settings","put","system","user_rotation","0");
    await delay(500);
    // Re-focus after the system keyboard/rotation changed the viewport.
    await evaluate("document.querySelector('.mobile-terminal-controls input').focus()");
    await shellLine("sleep 30");
    await wait(async()=> (await request("state")).foreground,"real foreground sleep");
    const interrupted=Date.now(); await click("Send Ctrl-C");
    await wait(async()=> !(await request("state")).foreground,"Ctrl-C interrupt",2000);
    await wait(async()=> (await terminalText()).includes("fixture> "),"shell prompt after interrupt");
    await shellLine('echo INTERRUPTED_"$?"');
    await wait(async()=> (await terminalText()).includes("INTERRUPTED_130"),"real interrupt status 130");
    check(Date.now()-interrupted<5000,"Foreground interrupt took too long");
    await evaluate("document.querySelector('.ssh-terminal').scrollIntoView({block:'center'})");
    await delay(250);
    const capture=await exec(process.env.VMANYWARE_ANDROID_ADB,["-P","5047","-s","emulator-5580","exec-out","screencap","-p"],{encoding:"buffer",maxBuffer:8*1024*1024,timeout:15000});
    await writeFile(join(directory,"ssh-terminal.png"),capture.stdout);
    await click("Disconnect");
    await wait(async()=> {const value=await request("state");return value.connections===0 && value.shellsClosed;},"explicit disconnect socket/shell cleanup");
    await open();
    await detach();await command("shell","am","force-stop","dev.vmanyware.client");
    await wait(async()=> {const value=await request("state");return value.connections===0 && value.shellsClosed;},"process-death socket/shell cleanup");
    await launch(); await inspect();await wait(()=>evaluate("Boolean(document.getElementById('ssh-username'))"),"persisted host trust after process restart");
    const before=await request("state");
    const rotated=await request("rotate");check(rotated.fingerprint!==metadata.fingerprint,"Fixture host key did not rotate");
    await inspect();await wait(()=>has("This host key differs"),"changed host refusal");
    check(!await evaluate("Boolean(document.getElementById('ssh-username'))"),"Changed host exposed authentication controls");
    const changed=await invoke("list_app_ssh_identity",{host:metadata.host,port:metadata.port,username:"fixture",passphrase:phrase});
    check(!changed.ok && changed.error.includes("host verification"),"Changed host allowed native identity selection");
    state=await request("state");check(state.authAttempts===before.authAttempts && state.authAttempts===3,"Changed host attempted another authentication");
    const report={platform:"android",target:"generated localhost SSH fixture via emulator host alias",authAttempts:state.authAttempts,channels:state.channels,ptys:state.ptys,sizes:state.sizes,inputBytes:state.inputBytes,connections:state.connections,shellsClosed:state.shellsClosed,checks:["independent fingerprint confirmation and fresh host verification", "wrong passphrase and untrusted/changed host refusal", "authentication-only check without channels", "Android IME input field -> Tauri -> real OS shell", "rotation-driven PTY resize and actual stty size", "foreground Ctrl-C and exit status 130", "disconnect/reconnect and process-death cleanup", "persisted trust after process restart"],limitations:["Generated host fixture, not a real guest or control-plane create journey", "ADB key-event injection is not full software-IME/composition or physical-device/iOS proof"]};
    await writeFile(join(directory,"ssh-report.json"),JSON.stringify(report,null,2)+"\n");
    console.log(`Android SSH/PTY passed: ${report.checks.join(", ")}`);
    return report;
  } catch(error) {
    // Only disposable fixture diagnostics, never a personal shell or key.
    if (diagnostics) console.error(diagnostics);
    try {
      console.error("Owned terminal diagnostics:", JSON.stringify({
        terminal:await terminalText(),
        focus:await evaluate("({tag:document.activeElement?.tagName,id:document.activeElement?.id,className:document.activeElement?.className,viewport:[innerWidth,innerHeight,visualViewport?.height]})"),
        fixture:await request("state")
      }));
    } catch { /* The owned WebView or server may already have closed. */ }
    throw error;
  } finally {
    if (child.exitCode===null && child.signalCode===null) {
      child.stdin.write(JSON.stringify({action:"stop"})+"\n");child.stdin.end();
      let timer;
      await Promise.race([exited,new Promise(resolve=>{timer=setTimeout(()=>{try{process.kill(-child.pid,"SIGKILL");}catch{}resolve();},3000);})]);
      clearTimeout(timer);
    }
    // Restore only the dedicated emulator's rotation/input fixture configuration.
    await command("shell","settings","put","system","user_rotation","0").catch(()=>{});
    await command("shell","settings","put","secure","show_ime_with_hard_keyboard","0").catch(()=>{});
  }
}
