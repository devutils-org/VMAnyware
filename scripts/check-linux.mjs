// Opt-in Linux check. Explicit local daemon, anonymous client, disposable source.
import { spawn } from "node:child_process";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
const target = process.argv[2] ?? "server";
if (!["server", "client"].includes(target)) {
  throw new Error("Expected server or client");
}
const image = target === "server"
  ? "rust@sha256:30c510257fde8a66d5b486c6c3459f9272e76546a7bd76613e6b4de38ebc9994"
  : "node@sha256:a22d4bb2667bdee11b2b8f7f912c55c2298d0b2deae627b854fecdf02038b4b9";
const commands = target === "server" ? [
  "rustc --version", "cargo --version",
  "rustup component add --toolchain 1.99.1 rustfmt clippy",
  "cargo fmt -p vmanyware-server -- --check",
  "cargo clippy --locked -p vmanyware-server --all-targets -- -D warnings",
  "cargo test --locked -p vmanyware-server",
] : [
  "node --version", "npm --version",
  "npm --prefix apps/client ci --no-audit --no-fund",
  "npm --prefix apps/client run build",
  "npm --prefix apps/client test",
];
const directory = await mkdtemp(join(tmpdir(), "vmanyware-linux-"));
const source = join(directory, "source"), config = join(directory, "docker");
await mkdir(source); await mkdir(config);
const environment = { PATH: "/usr/local/bin:/usr/bin:/bin", DOCKER_CONFIG: config };
const docker = ["--host", "unix:///var/run/docker.sock"];
const container = `vmanyware-linux-${randomUUID()}`;
let child;
const interrupt = () => child?.kill("SIGTERM");
process.on("SIGINT", interrupt); process.on("SIGTERM", interrupt);
function run(command, args, { input, capture = false, timeout = 600000 } = {}) {
  return new Promise((resolve, reject) => {
    child = spawn(command, args, { env: environment, stdio: ["pipe", "pipe", "pipe"] });
    const owned = child;
    const timer = setTimeout(() => owned.kill("SIGKILL"), timeout);
    let output = Buffer.alloc(0), errors = "";
    owned.stdout.on("data", bytes => {
      if (capture) output = Buffer.concat([output, bytes]); else process.stdout.write(bytes);
    });
    owned.stderr.on("data", bytes => {
      errors = (errors + bytes.toString()).slice(-8192);
      if (!capture) process.stderr.write(bytes);
    });
    owned.once("error", error => { clearTimeout(timer); reject(error); });
    owned.once("exit", (code, signal) => {
      clearTimeout(timer);
      if (child === owned) child = undefined;
      if (code === 0) resolve(output);
      else reject(new Error(`${command} failed (${code ?? signal}): ${errors}`));
    });
    owned.stdin.on("error", () => {});
    owned.stdin.end(input);
  });
}
try {
  const revision = (await run("git", ["rev-parse", "HEAD"], { capture: true })).toString().trim();
  const archive = await run("git", ["archive", "--format=tar", "HEAD"], { capture: true });
  await run("tar", ["-xf", "-", "-C", source], { input: archive });
  console.log(`Checking ${target} at committed revision ${revision} in isolated Linux arm64 (${image})`);
  await run("docker", [...docker, "version", "--format", "{{.Server.Os}} {{.Server.Arch}}"], { timeout: 10000 });
  console.log("Fetching the pinned public toolchain image (no login/personal Docker configuration)");
  await run("docker", [...docker, "pull", "--platform", "linux/arm64", image], { capture: true });
  await run("docker", [...docker, "run", "--rm", "--name", container,
    "--platform", "linux/arm64", "--cpus", "2", "--memory", "2g", "--pids-limit", "256",
    "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
    "--mount", `type=bind,src=${source},dst=/work`, "--workdir", "/work",
    "--env", "CARGO_BUILD_JOBS=2", image, "/bin/sh", "-eu", "-c", [
      "uname -sm", "cat /etc/os-release", ...commands,
    ].join("\n")]);
  console.log(`Linux ${target} check passed (container only; not GitHub Actions or native Linux desktop)`);
} finally {
  process.off("SIGINT", interrupt); process.off("SIGTERM", interrupt);
  await run("docker", [...docker, "rm", "--force", container], { capture: true, timeout: 10000 }).catch(() => {});
  await rm(directory, { recursive: true, force: true });
}
