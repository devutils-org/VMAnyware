// Host-only safety regression: fake ADB, no emulator/server/credential access.
import test from "node:test";
import assert from "node:assert/strict";
import {mkdtemp, writeFile, readFile, chmod, rm} from "node:fs/promises";
import {tmpdir} from "node:os";
import {join} from "node:path";
import {execFile} from "node:child_process";
import {promisify} from "node:util";
import {fileURLToPath} from "node:url";
const exec = promisify(execFile);
const source = fileURLToPath(new URL("./check.mjs",import.meta.url));
for (const mode of ["missing-apk", "wrong-avd", "changed-installed-apk"]) {
  test(`Keystore fixture refuses ${mode} before any app mutation`,async()=>{
    const root = await mkdtemp(join(tmpdir(),"vmanyware-keystore-guard-"));
    try {
      const log = join(root,"calls.jsonl"), apk = join(root,"owned.apk"), adb = join(root,"fake-adb");
      await writeFile(log,"");await writeFile(apk,"owned generated artifact; not an APK");
      await writeFile(adb,`#!${process.execPath}\nimport('node:fs').then(({appendFileSync})=>{const a=process.argv.slice(2);appendFileSync(process.env.VMANYWARE_GUARD_LOG,JSON.stringify(a)+'\\n');const c=a.slice(4).join(' ');if(c==='emu avd name')console.log(${JSON.stringify(mode==="wrong-avd"?"unrelated-emulator":"vmanyware-native-smoke")});else if(c==='shell getprop ro.kernel.qemu'||c==='shell getprop sys.boot_completed')console.log('1');else if(c==='shell pm path dev.vmanyware.client')console.log('package:/data/app/~~owned==/dev.vmanyware.client-owned==/base.apk');else if(c.startsWith('shell toybox sha256sum '))console.log('0'.repeat(64)+'  /data/app/owned/base.apk');else {console.log('Unexpected fake ADB operation');process.exitCode=9;}});\n`);
      await chmod(adb,0o700);
      const args=[source,join(root,"out"),"--reset-owned-app","--keystore",...(mode==="missing-apk"?[]:["--apk",apk])];
      await assert.rejects(exec(process.execPath,args,{timeout:5000,env:{PATH:process.env.PATH,VMANYWARE_ANDROID_ADB:adb,VMANYWARE_GUARD_LOG:log}}),error=>{
        assert.match(error.stderr, mode==="missing-apk"?/requires --apk/:mode==="wrong-avd"?/Wrong AVD/:/Installed APK differs/);return true;
      });
      const calls=(await readFile(log,"utf8")).trim().split("\n").filter(Boolean).map(JSON.parse);
      assert.equal(calls.length,mode==="missing-apk"?0:mode==="wrong-avd"?1:5);
      for(const args of calls){assert.deepEqual(args.slice(0,4),["-P","5047","-s","emulator-5580"]);assert.ok(!args.includes("clear")&&!args.includes("install")&&!args.includes("force-stop"));}
    } finally {await rm(root,{recursive:true,force:true});}
  });
}
