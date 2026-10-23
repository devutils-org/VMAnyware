// A read-only artifact plan by default. Running mutates only the explicitly guarded AVD.
import {execFile} from "node:child_process";
import {promisify} from "node:util";
import {readFile, writeFile, mkdir, stat} from "node:fs/promises";
import {createReadStream} from "node:fs";
import {resolve, dirname, join} from "node:path";
import {createHash} from "node:crypto";
import {fileURLToPath} from "node:url";
const exec = promisify(execFile);
const app = "dev.vmanyware.client", testApp = `${app}.test`;
const target = {avd:"vmanyware-native-smoke",serial:"emulator-5580",adbPort:5047,abi:"x86_64",requiresBootedEmulator:true};
const runner = `${testApp}/androidx.test.runner.AndroidJUnitRunner`;
const testClass = `${app}.IdentityVaultInstrumentedTest`;
const check = (value, message) => {if (!value) throw Error(message);};
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
async function artifact(sdk, path, expectedPackage, test) {
  path = resolve(path);
  const tools = join(sdk,"build-tools/36.0.0");
  const signature = (await exec(join(tools,"apksigner"),["verify","--verbose","--print-certs",path])).stdout;
  const certificateSha256 = signature.match(/Signer #1 certificate SHA-256 digest: ([0-9a-f]{64})/)?.[1];
  check(certificateSha256, "Missing verified APK signer");
  const badging = (await exec(join(tools,"aapt2"),["dump","badging",path])).stdout;
  check(badging.includes(`name='${expectedPackage}'`) && badging.includes("application-debuggable"), "Wrong APK package or non-debug artifact");
  const manifest = (await exec(join(tools,"aapt2"),["dump","xmltree",path,"--file","AndroidManifest.xml"])).stdout;
  if (test) {
    check(manifest.includes("androidx.test.runner.AndroidJUnitRunner") && manifest.includes('targetPackage') && manifest.includes(`"${app}"`), "Wrong instrumentation target or runner");
    check(/:testOnly\(0x[0-9a-f]+\)=true/.test(manifest), "Test APK must be marked test-only");
  } else {
    check(badging.includes("native-code: 'x86_64'"), "Main APK must be the x86_64 split");
    check(!/:testOnly\(0x[0-9a-f]+\)=true/.test(manifest), "Main APK cannot be test-only");
  }
  const digest = createHash("sha256");
  for await (const bytes of createReadStream(path)) digest.update(bytes);
  return {path,sha256:digest.digest("hex"),bytes:(await stat(path)).size,package:expectedPackage,certificateSha256};
}
export async function plan({sdk,apk,testApk}) {
  sdk = resolve(sdk);
  const main = await artifact(sdk,apk,app,false), instrumentation = await artifact(sdk,testApk,testApp,true);
  check(main.certificateSha256 === instrumentation.certificateSha256, "Main/test signer mismatch");
  const harnessSha256 = hash(Buffer.concat(await Promise.all(["check.mjs","keystore.mjs","keystore-plan.mjs"].map(name=>readFile(new URL(name,import.meta.url))))));
  const body = {schema:1,sdk,target,main,instrumentation,harnessSha256,operations:[
    "Require the already booted named QEMU emulator; never create/start an AVD or target a physical device",
    "Install/replace only the reviewed main and test APKs",
    "Clear only dev.vmanyware.client app data, generating a disposable application SSH identity",
    "Run actual production IPC save/read/process restart/forget; manual identity remains usable",
    "Run four scoped instrumented Keystore tests; generated test aliases are cleaned up",
    "Force-stop the owned app and remove the test APK; retain reports and the main APK"
  ],runtimeValidated:false};
  return {...body,planSha256:hash(JSON.stringify(body))};
}
async function run(file) {
  const reviewed = JSON.parse(await readFile(file,"utf8"));
  const current = await plan({sdk:reviewed.sdk,apk:reviewed.main.path,testApk:reviewed.instrumentation.path});
  check(JSON.stringify(current) === JSON.stringify(reviewed), "Plan or artifact changed; regenerate and review before any ADB call");
  const directory = resolve(dirname(file),"keystore-runtime");
  await mkdir(directory,{recursive:true});
  const adb = join(current.sdk,"platform-tools/adb");
  const command = async (...args) => (await exec(adb,["-P",String(target.adbPort),"-s",target.serial,...args],{timeout:args[0]==="shell" && args[2]==="instrument" ? 120000 : 15000,maxBuffer:1024*1024})).stdout.trim();
  check((await command("emu","avd","name")).split(/\r?\n/)[0] === target.avd, "Wrong AVD; no app mutation allowed");
  check(await command("shell","getprop","ro.kernel.qemu") === "1", "Physical device refused");
  check(await command("shell","getprop","ro.product.cpu.abi") === target.abi, "Wrong target ABI");
  check(await command("shell","getprop","sys.boot_completed") === "1", "Owned emulator is not booted");
  let installedTest = false;
  try {
    await command("install","-r",current.main.path);
    await command("install","-r","-t",current.instrumentation.path); installedTest = true;
    const native = await exec(process.execPath,[fileURLToPath(new URL("./check.mjs",import.meta.url)),join(directory,"native"),"--reset-owned-app","--keystore","--apk",current.main.path],{
      env:{...process.env,VMANYWARE_ANDROID_ADB:adb,VMANYWARE_ANDROID_HTTPS_DIRECT:"0"},timeout:180000,maxBuffer:1024*1024
    });
    await writeFile(join(directory,"native.log"),native.stdout+native.stderr);
    const result = await command("shell","am","instrument","-w","-r","-e","class",testClass,"-e","vmanywareOwnedVault",`${target.avd}:${target.serial}:reset-approved`,runner);
    await writeFile(join(directory,"instrumentation.log"),result+"\n");
    check(/OK \(4 tests\)/.test(result) && /INSTRUMENTATION_CODE: -1/.test(result) && !/FAILURES!!!|INSTRUMENTATION_FAILED/.test(result), "Exactly four successful instrumented cases required");
    const nativeReport = JSON.parse(await readFile(join(directory,"native/report.json"),"utf8"));
    check(nativeReport.apkSha256 === current.main.sha256 && nativeReport.keystore?.checks.length === 5, "Native report lacks exact APK/Keystore acceptance");
    await writeFile(join(directory,"report.json"),JSON.stringify({planSha256:current.planSha256,apkSha256:current.main.sha256,testApkSha256:current.instrumentation.sha256,target,keystore:nativeReport.keystore,instrumentedTests:4,limitations:["Owned emulator only; no physical security-level/biometric proof", "No real Proxmox, VM creation or system-source SSH/PTY acceptance"]},null,2)+"\n");
    console.log(`Owned Android Keystore runtime passed; report: ${join(directory,"report.json")}`);
  } finally {
    await command("shell","am","force-stop",app).catch(()=>{});
    if (installedTest) await command("uninstall",testApp).catch(()=>{});
  }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const value = name => {const i=args.indexOf(name);check(i>=0 && args[i+1] && !args[i+1].startsWith("--"),`Supply ${name}`);return args[i+1];};
  if (args.includes("--plan-only")) {
    check(!args.includes("--run-approved"), "Select only one mode");
    const result = await plan({sdk:value("--sdk"),apk:value("--apk"),testApk:value("--test-apk")});
    const file = resolve(value("--output"));await mkdir(dirname(file),{recursive:true});await writeFile(file,JSON.stringify(result,null,2)+"\n");
    console.log(`Read-only plan saved: ${file}\nPlan SHA-256: ${result.planSha256}\nNo ADB call, emulator start, installation or credential operation occurred.`);
  } else {
    check(args.includes("--run-approved") && args.includes("--reset-owned-app"), "Review a --plan-only result, then explicitly supply --run-approved PLAN --reset-owned-app");
    await run(value("--run-approved"));
  }
}
