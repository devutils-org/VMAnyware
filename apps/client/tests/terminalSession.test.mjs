import { test } from "node:test";
import assert from "node:assert/strict";
import { TerminalSession } from "../src/terminalSession.ts";
const tick = () => new Promise(resolve => setImmediate(resolve));
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function harness() {
  const calls = [], rendered = [], failures = [], exits = [];
  const reads = [], writes = [];
  let closed = 0;
  const session = new TerminalSession("owned", async (command, args) => {
    calls.push({ command, args });
    if (command === "read_ssh_session") { const request = deferred(); reads.push(request); return request.promise; }
    if (command === "write_ssh_session") { const request = deferred(); writes.push(request); return request.promise; }
  }, async bytes => { rendered.push([...bytes]); }, status => exits.push(status), () => { closed++; }, message => failures.push(message));
  return { session, calls, rendered, failures, exits, reads, writes, closed: () => closed };
}
test("binary output waits for renderer consumption before reading again", async () => {
  const reads = [], blocks = [], rendered = [];
  const session = new TerminalSession("one", async command => {
    if (command !== "read_ssh_session") return;
    const request = deferred(); reads.push(request); return request.promise;
  }, bytes => { rendered.push([...bytes]); const block = deferred(); blocks.push(block); return block.promise; }, () => {}, () => {}, () => {});
  session.start(); session.start();
  assert.equal(reads.length, 1);
  reads[0].resolve({ type: "data", bytes: [0xe2, 0x82] });
  await tick();
  assert.equal(reads.length, 1);
  assert.deepEqual(rendered, [[0xe2, 0x82]]);
  blocks[0].resolve(); await tick();
  assert.equal(reads.length, 2);
  reads[1].resolve({ type: "stderr", bytes: [0xac, 0, 255, 3] }); await tick();
  assert.deepEqual(rendered[1], [0xac, 0, 255, 3]);
  session.stop(); blocks[1].resolve(); await tick();
  assert.equal(reads.length, 2);
});
test("stale reads and failures after stop cannot affect a new generation", async () => {
  for (const reject of [false, true]) {
    const h = harness(); h.session.start(); h.session.stop(); h.session.stop();
    if (reject) h.reads[0].reject(new Error("late private transport error"));
    else h.reads[0].resolve({ type: "data", bytes: [65] });
    await tick();
    assert.deepEqual(h.rendered, []); assert.deepEqual(h.failures, []);
    assert.equal(h.calls.filter(call => call.command === "close_ssh_session").length, 1);
  }
});
test("input is serialized in bounded chunks and resize storms coalesce", async () => {
  const h = harness();
  const bytes = Uint8Array.from({ length: 65537 }, (_, n) => n % 256);
  h.session.write(bytes);
  assert.equal(h.writes.length, 1);
  for (let n = 1; n <= 100; n++) h.session.resize(n, 24);
  for (let n = 0; n < 3; n++) { h.writes[n].resolve(); await tick(); }
  const sent = h.calls.filter(call => call.command === "write_ssh_session");
  assert.deepEqual(sent.map(call => call.args.bytes.length), [32768, 32768, 1]);
  assert.deepEqual(sent.flatMap(call => call.args.bytes), [...bytes]);
  const resize = h.calls.filter(call => call.command === "resize_ssh_session");
  assert.equal(resize.length, 1); assert.equal(resize[0].args.cols, 100);
  assert.ok(h.calls.every(call => call.args.sessionId === "owned"));
  h.session.stop();
});
test("input overflow closes visibly instead of silently dropping input", async () => {
  const h = harness();
  h.session.write(new Uint8Array(128 * 1024));
  h.session.write(new Uint8Array([3]));
  assert.equal(h.failures.length, 1);
  assert.match(h.failures[0], /buffer limit/);
  h.writes[0].resolve(); await tick();
  assert.equal(h.calls.filter(call => call.command === "write_ssh_session").length, 1);
  assert.equal(h.calls.filter(call => call.command === "close_ssh_session").length, 1);
});
test("remote exit is observed before EOF and final close releases native ownership", async () => {
  const h = harness(); h.session.start();
  for (const [n, event] of [{ type: "exit", status: 42 }, { type: "eof" }, { type: "closed" }].entries()) {
    h.reads[n].resolve(event); await tick();
  }
  assert.deepEqual(h.exits, [42]); assert.equal(h.closed(), 1);
  assert.equal(h.reads.length, 3); assert.deepEqual(h.failures, []);
  assert.equal(h.calls.filter(call => call.command === "close_ssh_session").length, 1);
});
test("read failure has a fixed message and closes the owned connection", async () => {
  const h = harness(); h.session.start();
  h.reads[0].reject(new Error("private upstream body")); await tick();
  assert.equal(h.failures.length, 1); assert.ok(!h.failures[0].includes("private"));
  assert.equal(h.calls.filter(call => call.command === "close_ssh_session").length, 1);
});
test("a full native queue retries the same bytes without reordering", async () => {
  const sent = []; let attempts = 0;
  const session = new TerminalSession("one", async (command, args) => {
    if (command === "write_ssh_session") {
      if (++attempts === 1) throw "SSH input queue is full; retry after pending input is sent";
      sent.push(args.bytes);
    }
  }, async () => {}, () => {}, () => {}, message => assert.fail(message));
  session.write(new Uint8Array([3, 0, 255])); session.write(new Uint8Array([65]));
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.deepEqual(sent, [[3, 0, 255], [65]]);
  session.stop();
});
