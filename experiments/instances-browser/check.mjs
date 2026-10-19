// Real React/browser interaction with simulated IPC; never starts native SSH/provider work.
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import { join } from "node:path";

if (process.platform !== "linux") throw new Error("This opt-in check requires Linux Firefox");
const root = fileURLToPath(new URL("../../", import.meta.url));
const require = createRequire(new URL("../../apps/client/package.json", import.meta.url));
const { createServer } = await import(pathToFileURL(require.resolve("vite")).href);
const directory = await mkdtemp("/tmp/vmanyware-instance-browser-");
let server, browser;
let diagnostics = "";
let timer;
try {
  const profile = join(directory, "profile");
  await mkdir(profile);
  await writeFile(join(profile, "user.js"), [
    ["browser.shell.checkDefaultBrowser", false], ["browser.startup.homepage", "about:blank"],
    ["browser.startup.firstrunSkipsHomepage", true], ["browser.aboutwelcome.enabled", false],
    ["datareporting.policy.dataSubmissionEnabled", false], ["toolkit.telemetry.enabled", false],
    ["network.captive-portal-service.enabled", false], ["network.connectivity-service.enabled", false],
    ["extensions.update.enabled", false], ["browser.region.network.url", ""],
  ].map(([key, value]) => `user_pref(${JSON.stringify(key)}, ${JSON.stringify(value)});`).join("\n"));
  const fixture = await readFile(new URL(process.argv.includes("--app-ssh") ? "./app-ssh-fixture.js" : process.argv.includes("--remote-connection") ? "./remote-fixture.js" : "./fixture.js", import.meta.url), "utf8");
  let resolveResult, rejectResult;
  const result = new Promise((resolve, reject) => { resolveResult = resolve; rejectResult = reject; });
  server = await createServer({
    root: join(root, "apps/client"),
    server: { host: "127.0.0.1", port: 0, strictPort: false },
    plugins: [{
      name: "isolated-instance-ipc-fixture",
      transformIndexHtml() { return [{ tag: "script", children: fixture, injectTo: "head-prepend" }]; },
      configureServer(vite) {
        vite.middlewares.use((request, response, next) => {
          if (request.url !== "/__instance_fixture/result" || request.method !== "POST") return next();
          let body = "";
          request.on("data", chunk => { body += chunk; if (body.length > 8192) request.destroy(); });
          request.on("end", () => {
            try {
              const report = JSON.parse(body);
              response.end("received");
              report.ok ? resolveResult(report) : rejectResult(new Error(report.error));
            } catch (error) { rejectResult(error); response.end("invalid result"); }
          });
        });
      },
    }],
  });
  await server.listen();
  const url = `http://127.0.0.1:${server.httpServer.address().port}/`;
  browser = spawn("/usr/bin/firefox", ["--headless", "--no-remote", "--profile", profile, url], {
    detached: true,
    env: { PATH: process.env.PATH, TMPDIR: directory, XDG_RUNTIME_DIR: directory, MOZ_CRASHREPORTER_DISABLE: "1", MOZ_CRASHREPORTER_NO_REPORT: "1" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  const collect = chunk => { diagnostics = (diagnostics + chunk).slice(-8192); };
  browser.stdout.on("data", collect); browser.stderr.on("data", collect);
  browser.once("error", rejectResult);
  browser.once("exit", (code, signal) => rejectResult(new Error(`Fixture browser exited: ${code ?? signal}`)));
  timer = setTimeout(() => rejectResult(new Error("Instance browser fixture exceeded 30 seconds")), 30_000);
  const report = await result;
  console.log(`Instance browser fixture passed: ${report.checks.join(", ")}`);
} catch (error) {
  if (diagnostics) console.error(diagnostics);
  throw error;
} finally {
  clearTimeout(timer);
  if (browser?.pid && browser.exitCode === null && browser.signalCode === null) {
    const exited = new Promise(resolve => browser.once("exit", resolve));
    try { process.kill(-browser.pid, "SIGTERM"); } catch { /* Already exited. */ }
    let killTimer;
    await Promise.race([exited, new Promise(resolve => { killTimer = setTimeout(() => { try { process.kill(-browser.pid, "SIGKILL"); } catch {} resolve(); }, 3000); })]);
    clearTimeout(killTimer);
  }
  await server?.close();
  await rm(directory, { recursive: true, force: true });
}
