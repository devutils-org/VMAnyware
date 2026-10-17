// Run only the owned, compiled native fixture with a cleared environment.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
if (process.platform !== "darwin") throw new Error("Native terminal smoke requires macOS");
const binary = fileURLToPath(new URL("../../target/release/examples/terminal_smoke", import.meta.url));
await new Promise((resolve, reject) => {
  const child = spawn(binary, [], { env: { PATH: "/usr/bin:/bin" }, stdio: ["ignore", "ignore", "pipe"] });
  let output = "";
  const timer = setTimeout(() => child.kill("SIGKILL"), 60000);
  child.stderr.on("data", bytes => {
    process.stderr.write(bytes);
    output = (output + bytes.toString()).slice(-8192);
  });
  child.once("error", error => { clearTimeout(timer); reject(error); });
  child.once("exit", (code, signal) => {
    clearTimeout(timer);
    if (code === 0 && output.includes("Native Tauri smoke: passed")) resolve();
    else reject(new Error(`Native fixture failed: ${code ?? signal}`));
  });
});
