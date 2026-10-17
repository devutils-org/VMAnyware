export type TerminalEvent =
  | { type: "data" | "stderr"; bytes: number[] }
  | { type: "exit"; status: number }
  | { type: "eof" | "closed" | "failed" };
export type NativeInvoke = (command: string, args: Record<string, unknown>) => Promise<unknown>;
const FULL = "SSH input queue is full; retry after pending input is sent";
const MAX_QUEUED = 128 * 1024;

/** One renderer/session generation. No DOM, credentials or automatic connection. */
export class TerminalSession {
  private stopped = false;
  private started = false;
  private writing = false;
  private queuedBytes = 0;
  private queue: Uint8Array[] = [];
  private size: { cols: number; rows: number } | null = null;
  private readonly id: string;
  private readonly invoke: NativeInvoke;
  private readonly render: (bytes: Uint8Array) => Promise<void>;
  private readonly onExit: (status: number) => void;
  private readonly onClosed: () => void;
  private readonly onFailure: (message: string) => void;
  constructor(
    id: string,
    invoke: NativeInvoke,
    render: (bytes: Uint8Array) => Promise<void>,
    onExit: (status: number) => void,
    onClosed: () => void,
    onFailure: (message: string) => void,
  ) {
    this.id = id; this.invoke = invoke; this.render = render;
    this.onExit = onExit; this.onClosed = onClosed; this.onFailure = onFailure;
  }
  start() {
    if (this.started || this.stopped) return;
    this.started = true;
    void this.read();
  }
  stop(closeNative = true) {
    if (this.stopped) return;
    this.stopped = true;
    this.queue = [];
    this.size = null;
    if (closeNative) void this.invoke("close_ssh_session", { sessionId: this.id }).catch(() => {});
  }
  write(bytes: Uint8Array) {
    if (this.stopped || bytes.length === 0) return;
    if (bytes.length + this.queuedBytes > MAX_QUEUED) {
      this.fail("Terminal input exceeded its buffer limit. The connection was closed; reconnect before retrying.");
      return;
    }
    this.queuedBytes += bytes.length;
    for (let offset = 0; offset < bytes.length; offset += 32768) this.queue.push(bytes.slice(offset, offset + 32768));
    void this.flush();
  }
  resize(cols: number, rows: number) {
    if (this.stopped || !Number.isInteger(cols) || !Number.isInteger(rows) || cols < 1 || rows < 1 || cols > 500 || rows > 500) return;
    this.size = { cols, rows }; // Coalesce resize storms instead of growing a queue.
    void this.flush();
  }
  private fail(message: string) {
    if (this.stopped) return;
    this.stop();
    this.onFailure(message);
  }
  private async read() {
    try {
      while (!this.stopped) {
        const event = await this.invoke("read_ssh_session", { sessionId: this.id }) as TerminalEvent | null;
        if (this.stopped) return;
        if (!event) continue;
        if (event.type === "data" || event.type === "stderr") {
          // Await xterm's parsed-write callback before pulling another block.
          await this.render(new Uint8Array(event.bytes));
        } else if (event.type === "exit") this.onExit(event.status);
        else if (event.type === "closed") { this.stop(); this.onClosed(); }
        else if (event.type === "failed") this.fail("The SSH terminal transport failed. Reconnect to start a fresh session.");
      }
    } catch {
      this.fail("Cannot read this SSH session. The connection was closed; reconnect to retry.");
    }
  }
  private async send(command: string, args: Record<string, unknown>) {
    const deadline = Date.now() + 1000;
    while (!this.stopped) {
      try { await this.invoke(command, { sessionId: this.id, ...args }); return; }
      catch (error) {
        if (error !== FULL || Date.now() >= deadline) throw error;
        await new Promise(resolve => setTimeout(resolve, 10));
      }
    }
  }
  private async flush() {
    if (this.writing || this.stopped) return;
    this.writing = true;
    try {
      while (!this.stopped && (this.queue.length || this.size)) {
        // Input first so a resize storm cannot starve Ctrl-C.
        const bytes = this.queue.shift();
        if (bytes) {
          await this.send("write_ssh_session", { bytes: Array.from(bytes) });
          this.queuedBytes -= bytes.length;
        } else if (this.size) {
          const size = this.size;
          this.size = null;
          await this.send("resize_ssh_session", size);
        }
      }
    } catch {
      this.fail("Cannot send terminal input or resize. The connection was closed; reconnect to retry.");
    } finally { this.writing = false; }
  }
}
