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
  for (const failRenderer of [false, true]) {
    let blocked = true;
    let pending = false;
    const releases = [];
    server = await createServer({
      root: join(root, "apps/client"),
      server: { host: "127.0.0.1", port: 0, strictPort: false },
      plugins: [{
        name: "isolated-terminal-module-gate",
        configureServer(vite) {
          vite.middlewares.use((request, response, next) => {
            const path = request.url?.split("?")[0];
            if (path === "/__terminal_fixture/pending") {
              response.setHeader("Content-Type", "application/json");
              response.end(JSON.stringify({ pending }));
            } else if (path === "/__terminal_fixture/release") {
              blocked = false; releases.splice(0).forEach(resolve => resolve());
              response.end("released");
            } else if (path === "/src/SshTerminal.tsx") {
              if (failRenderer) { response.statusCode = 503; response.end("fixture renderer failure"); }
              else if (blocked) { pending = true; releases.push(next); }
              else next();
            } else next();
          });
        },
        transform(code, id) {
          if (id.endsWith("/src/SshTerminal.tsx")) return `${code}\nglobalThis.__terminalFixtureLoaded = true;`;
        },
      }],
    });
    await server.listen();
    const address = server.httpServer.address();
    try {
      await run(binary, [`http://127.0.0.1:${address.port}/?rendererFailure=${Number(failRenderer)}`, join(root, "experiments/terminal-webkit/fixture.js")], 40000);
    } finally {
      blocked = false; releases.splice(0).forEach(resolve => resolve());
      await server.close(); server = undefined;
    }
  }
} finally {
  await server?.close();
  await rm(directory, { recursive: true, force: true });
}
