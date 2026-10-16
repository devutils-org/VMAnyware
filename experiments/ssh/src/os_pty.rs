//! Native Unix shell fixture only. Never accepts an external endpoint or command.
use super::*;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};

pub(super) struct Shell {
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    child: Box<dyn Child + Send + Sync>,
    reader: Option<std::thread::JoinHandle<()>>,
    stop_reader: Arc<std::sync::atomic::AtomicBool>,
    forwarder: JoinHandle<()>,
    _directory: tempfile::TempDir,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        cols,
        rows,
        pixel_width: 0,
        pixel_height: 0,
    }
}

impl Shell {
    pub(super) fn start(
        dimensions: (u16, u16),
        channel: ChannelId,
        session: server::Handle,
    ) -> Self {
        let pair = native_pty_system()
            .openpty(size(dimensions.0, dimensions.1))
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let mut command = CommandBuilder::new("/bin/sh");
        command.arg("-i");
        command.env_clear();
        command.env("PATH", "/usr/bin:/bin");
        command.env("TERM", "xterm-256color");
        command.env("PS1", "fixture> ");
        command.cwd(directory.path());
        let child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let (output, mut receiver) = mpsc::unbounded_channel();
        // Nonblocking input permits deterministic thread shutdown even on failures.
        let fd = pair.master.as_raw_fd().unwrap();
        // SAFETY: fd belongs to this live fixture master; preserve existing flags.
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            assert!(flags >= 0);
            assert_eq!(libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK), 0);
        }
        let stop_reader = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop = stop_reader.clone();
        let reader = std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            while !stop.load(Ordering::SeqCst) {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if output.send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        let forwarder = tokio::spawn(async move {
            while let Some(bytes) = receiver.recv().await {
                if !matches!(
                    timeout(Duration::from_secs(1), session.data(channel, bytes)).await,
                    Ok(Ok(()))
                ) {
                    break;
                }
            }
        });
        Self {
            master: Some(pair.master),
            writer: Some(writer),
            child,
            reader: Some(reader),
            stop_reader,
            forwarder,
            _directory: directory,
        }
    }

    pub(super) fn write(&mut self, data: &[u8]) {
        self.writer.as_mut().unwrap().write_all(data).unwrap();
    }
    pub(super) fn resize(&self, cols: u16, rows: u16) {
        self.master
            .as_ref()
            .unwrap()
            .resize(size(cols, rows))
            .unwrap();
    }

    pub(super) fn close(&mut self) {
        let Some(master) = self.master.take() else {
            return;
        };
        // Only fixture-created process groups: current foreground job and shell.
        // SAFETY: these calls use this owned PTY's fd and generated child PID.
        unsafe {
            if let Some(fd) = master.as_raw_fd() {
                let foreground = libc::tcgetpgrp(fd);
                if foreground > 0 && foreground != libc::getpgrp() {
                    libc::killpg(foreground, libc::SIGKILL);
                }
            }
            if let Some(pid) = self.child.process_id() {
                let group = pid as libc::pid_t;
                if group > 0 && group != libc::getpgrp() {
                    libc::killpg(group, libc::SIGKILL);
                }
            }
        }
        self.stop_reader.store(true, Ordering::SeqCst);
        self.forwarder.abort();
        self.writer.take();
        drop(master);
        let _ = self.child.kill();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        loop {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "fixture child did not terminate after SIGKILL"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.close();
    }
}

async fn output_until(channel: &mut Channel<client::Msg>, expected: &str) -> String {
    timeout(Duration::from_secs(2), async {
        let mut output = Vec::new();
        loop {
            match channel.wait().await {
                Some(ChannelMsg::Data { data }) => output.extend_from_slice(&data),
                Some(ChannelMsg::Close) | None => {
                    panic!("channel closed before expected fixture output")
                }
                _ => {}
            }
            assert!(output.len() < 65536, "fixture output bound exceeded");
            let text = String::from_utf8_lossy(&output);

            if text.contains(expected) {
                return text.into_owned();
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("expected fixture output was not received: {expected}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_shell_stdout_terminal_resize_and_ctrl_c_interrupt() {
    timeout(LIMIT, async {
        let mut fixture = Fixture::with_os_shell(true).await;
        for _ in 0..2 {
            let mut handle = connect(
                &fixture,
                Some(fixture.trusted_host.clone()),
                Arc::new(AtomicUsize::new(0)),
            )
            .await
            .unwrap();
            assert!(authenticate(&mut handle, fixture.user_key.clone()).await);
            assert_eq!(fixture.next_event().await, Event::Auth(true));
            let mut channel = handle.channel_open_session().await.unwrap();
            channel
                .request_pty(true, "xterm-256color", 80, 24, 0, 0, &[])
                .await
                .unwrap();
            channel_success(&mut channel).await;
            assert!(matches!(fixture.next_event().await, Event::Pty { .. }));
            channel.request_shell(true).await.unwrap();
            channel_success(&mut channel).await;
            assert_eq!(fixture.next_event().await, Event::Shell);
            output_until(&mut channel, "fixture> ").await;
            channel
                .data(&b"stty -echo; printf 'READY\\n'\n"[..])
                .await
                .unwrap();
            output_until(&mut channel, "\r\nREADY\r\n").await;
            channel
                .data(&b"printf 'STDOUT_OK\\n'; stty size\n"[..])
                .await
                .unwrap();
            let output = output_until(&mut channel, "24 80").await;
            assert!(output.contains("STDOUT_OK"));
            channel.window_change(132, 43, 0, 0).await.unwrap();
            channel.data(&b"stty size\n"[..]).await.unwrap();
            output_until(&mut channel, "43 132").await;
            channel
                .data(&b"printf 'SLEEPING\\n'; sleep 5\n"[..])
                .await
                .unwrap();
            output_until(&mut channel, "SLEEPING\r\n").await;
            // Wait until shell has handed the terminal to its foreground child.
            let shell = fixture
                .shells
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .last()
                .unwrap()
                .clone();
            timeout(Duration::from_secs(1), async {
                loop {
                    let foreground_is_child = {
                        let shell = shell.lock().unwrap_or_else(|poison| poison.into_inner());
                        let fd = shell.master.as_ref().unwrap().as_raw_fd().unwrap();
                        // SAFETY: fd belongs to the live fixture PTY.
                        unsafe {
                            let foreground = libc::tcgetpgrp(fd);
                            foreground > 0
                                && foreground != shell.child.process_id().unwrap() as libc::pid_t
                        }
                    };
                    if foreground_is_child {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            let interrupted_at = tokio::time::Instant::now();
            channel.data(&b"\x03"[..]).await.unwrap();
            output_until(&mut channel, "fixture> ").await;
            channel
                .data(&b"printf 'INTERRUPTED:%s\\n' \"$?\"\n"[..])
                .await
                .unwrap();
            output_until(&mut channel, "INTERRUPTED:130").await;
            assert!(interrupted_at.elapsed() < Duration::from_secs(2));
            channel.close().await.unwrap();
            // Drain protocol events to ensure close reached server and reaped shell.
            loop {
                if fixture.next_event().await == Event::Closed {
                    break;
                }
            }
            handle
                .disconnect(Disconnect::ByApplication, "fixture complete", "en")
                .await
                .unwrap();
        }
    })
    .await
    .unwrap();
}
