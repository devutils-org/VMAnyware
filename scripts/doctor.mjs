import { spawnSync } from "node:child_process";

const checks = [
  ["Node.js", "node", ["--version"]],
  ["npm", "npm", ["--version"]],
  ["Rust", "rustc", ["--version"]],
  ["Cargo", "cargo", ["--version"]],
];

if (process.platform === "darwin") {
  checks.push(["Apple developer directory", "xcode-select", ["-p"]]);
}

let missing = false;
for (const [label, command, args] of checks) {
  const result = spawnSync(command, args, { encoding: "utf8", timeout: 5000 });
  if (result.error || result.status !== 0) {
    console.log(`${label}: unavailable`);
    missing = true;
  } else {
    console.log(`${label}: ${result.stdout.trim()}`);
  }
}
console.log("This checks tool availability only, not native/mobile build readiness.");
process.exitCode = missing ? 1 : 0;
