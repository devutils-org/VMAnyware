// macOS-only opt-in renderer check. Uses an ephemeral WebKit store and fake IPC.
import { mkdtemp, rm } from "node:fs/promises";
import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import { join } from "node:path";
if (process.platform !== "darwin") throw new Error("This check requires macOS WebKit and swiftc");
const root = fileURLToPath(new URL("../../", import.meta.url));
const require = createRequire(new URL("../../apps/client/package.json", import.meta.url));
const { createServer } = await import(pathToFileURL(require.resolve("vite")).href);
const directory = await mkdtemp("/private/tmp/pl-webkit-");
let server;
async function run(command, args, deadline) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: root, stdio: ["ignore", "inherit", "inherit"] });
    const timer = setTimeout(() => child.kill("SIGKILL"), deadline);
    child.once("error", error => { clearTimeout(timer); reject(error); });
    child.once("exit", (code, signal) => {
      clearTimeout(timer);
      code === 0 ? resolve() : reject(new Error(`Fixture child failed: ${code ?? signal}`));
    });
  });
}
try {
  const binary = join(directory, "webkit-smoke");
  await run("/usr/bin/swiftc", ["experiments/terminal-webkit/main.swift", "-o", binary], 60000);
  server = await createServer({ root: join(root, "apps/client"), server: { host: "127.0.0.1", port: 0, strictPort: false } });
  await server.listen();
  const address = server.httpServer.address();
  await run(binary, [`http://127.0.0.1:${address.port}`, join(root, "experiments/terminal-webkit/fixture.js")], 40000);
} finally {
  await server?.close();
  await rm(directory, { recursive: true, force: true });
}
