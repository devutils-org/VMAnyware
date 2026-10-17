//! Native Unix shell fixture only. Never accepts an external endpoint or command.
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use russh::{server, ChannelId};
use std::io::{Read, Write};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::{sync::mpsc, task::JoinHandle, time::timeout};

pub(crate) struct Shell {
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
    pub(crate) fn start(
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
        let (output, mut receiver) = mpsc::channel(8);
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
                        let mut pending = buffer[..count].to_vec();
                        loop {
                            if stop.load(Ordering::SeqCst) {
                                return;
                            }
                            match output.try_send(pending) {
                                Ok(()) => break,
                                Err(mpsc::error::TrySendError::Closed(_)) => return,
                                Err(mpsc::error::TrySendError::Full(bytes)) => {
                                    pending = bytes;
                                    std::thread::sleep(Duration::from_millis(5));
                                }
                            }
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

    pub(crate) fn write(&mut self, data: &[u8]) {
        self.writer.as_mut().unwrap().write_all(data).unwrap();
    }
    pub(crate) fn resize(&self, cols: u16, rows: u16) {
        self.master
            .as_ref()
            .unwrap()
            .resize(size(cols, rows))
            .unwrap();
    }

    pub(crate) fn foreground_child(&self) -> Option<u32> {
        let fd = self.master.as_ref()?.as_raw_fd()?;
        // SAFETY: fd is owned by the live fixture PTY.
        let foreground = unsafe { libc::tcgetpgrp(fd) };
        (foreground > 0 && Some(foreground as u32) != self.child.process_id())
            .then_some(foreground as u32)
    }
    pub(crate) fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }
    pub(crate) fn is_closed(&self) -> bool {
        self.master.is_none()
    }

    pub(crate) fn close(&mut self) {
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
