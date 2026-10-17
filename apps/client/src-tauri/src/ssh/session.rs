//! Pull-based, bounded binary PTY transport. No terminal renderer or forwarding.
use super::agent::{AgentManager, Authenticated};
use russh::{client, Channel, ChannelMsg};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use uuid::Uuid;

const CAPACITY: usize = 8;
const OUTPUT_STALL: Duration = Duration::from_secs(3);
const MAX_DATA: usize = 32 * 1024;
const UNAVAILABLE: &str = "SSH session unavailable or closed";
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    Data { bytes: Vec<u8> },
    Stderr { bytes: Vec<u8> },
    Exit { status: u32 },
    Eof,
    Closed,
    Failed,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    id: String,
}
enum Input {
    Data(Vec<u8>),
    Resize(u32, u32),
}
struct Output {
    receiver: mpsc::Receiver<Event>,
    failed: Arc<AtomicBool>,
}
struct Live {
    input: mpsc::Sender<Input>,
    output: Arc<AsyncMutex<Output>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Live {
    fn drop(&mut self) {
        self.task.abort();
    }
}
struct Slot {
    id: String,
    selection: String,
    live: Option<Live>,
}
#[derive(Clone)]
pub struct SessionManager {
    agents: AgentManager,
    owners: Arc<Mutex<HashMap<String, Slot>>>,
    starts: Arc<tokio::sync::Semaphore>,
}
impl SessionManager {
    fn current(&self, owner: &str, id: &str) -> Result<(), String> {
        let owners = self.owners.lock().map_err(|_| UNAVAILABLE)?;
        if owners.get(owner).is_some_and(|slot| slot.id == id) {
            Ok(())
        } else {
            Err(UNAVAILABLE.into())
        }
    }
    pub fn new(agents: AgentManager) -> Self {
        Self {
            agents,
            owners: Arc::default(),
            starts: Arc::new(tokio::sync::Semaphore::new(CAPACITY)),
        }
    }
    pub fn forget_owner(&self, owner: &str) {
        let removed = self
            .owners
            .lock()
            .ok()
            .and_then(|mut owners| owners.remove(owner));
        if let Some(slot) = removed {
            self.agents.cancel_selection(owner, &slot.selection);
        }
    }
    pub fn close(&self, owner: &str, id: &str) -> Result<(), String> {
        let mut owners = self.owners.lock().map_err(|_| UNAVAILABLE)?;
        if owners.get(owner).is_none_or(|slot| slot.id != id) {
            return Err(UNAVAILABLE.into());
        }
        let slot = owners.remove(owner).ok_or(UNAVAILABLE)?;
        drop(owners);
        self.agents.cancel_selection(owner, &slot.selection);
        Ok(())
    }
    pub async fn start(
        &self,
        owner: &str,
        selection: &str,
        fingerprint: &str,
        cols: u32,
        rows: u32,
    ) -> Result<Started, String> {
        dimensions(cols, rows)?;
        if selection.len() > 64 || fingerprint.len() > 128 {
            return Err(UNAVAILABLE.into());
        }
        let _permit = self
            .starts
            .try_acquire()
            .map_err(|_| "Too many SSH session starts")?;
        let id = Uuid::new_v4().to_string();
        let old = {
            let mut owners = self.owners.lock().map_err(|_| UNAVAILABLE)?;
            if !owners.contains_key(owner) && owners.len() >= CAPACITY {
                return Err("Too many SSH sessions".into());
            }
            owners.insert(
                owner.into(),
                Slot {
                    id: id.clone(),
                    selection: selection.into(),
                    live: None,
                },
            )
        };
        if let Some(old) = old {
            // A new listing normally produced a distinct selection. Do not cancel
            // the selection being used by this replacement start.
            if old.selection != selection {
                self.agents.cancel_selection(owner, &old.selection);
            }
        }
        let established = tokio::time::timeout(Duration::from_secs(10), async {
            let connection = self
                .agents
                .connect_authenticated(owner, selection, fingerprint)
                .await?;
            self.current(owner, &id)?;
            let mut channel = connection
                .handle
                .channel_open_session()
                .await
                .map_err(|_| "SSH session channel refused")?;
            self.current(owner, &id)?;
            channel
                .request_pty(true, "xterm-256color", cols, rows, 0, 0, &[])
                .await
                .map_err(|_| "SSH PTY request failed")?;
            accepted(&mut channel).await?;
            self.current(owner, &id)?;
            channel
                .request_shell(true)
                .await
                .map_err(|_| "SSH shell request failed")?;
            accepted(&mut channel).await?;
            Ok::<_, String>((connection, channel))
        })
        .await
        .map_err(|_| "SSH session start timed out".to_owned())
        .and_then(|result| result);
        let (connection, channel) = match established {
            Ok(value) => value,
            Err(error) => {
                let _ = self.close(owner, &id);
                return Err(error);
            }
        };
        let mut owners = self.owners.lock().map_err(|_| UNAVAILABLE)?;
        let slot = owners
            .get_mut(owner)
            .filter(|slot| slot.id == id)
            .ok_or(UNAVAILABLE)?;
        let (input, commands) = mpsc::channel(CAPACITY);
        let (events, output) = mpsc::channel(CAPACITY);
        let failed = Arc::new(AtomicBool::new(false));
        let task = tokio::spawn(run(connection, channel, commands, events, failed.clone()));
        slot.live = Some(Live {
            input,
            output: Arc::new(AsyncMutex::new(Output {
                receiver: output,
                failed,
            })),
            task,
        });
        Ok(Started { id })
    }
    fn input(&self, owner: &str, id: &str, input: Input) -> Result<(), String> {
        let owners = self.owners.lock().map_err(|_| UNAVAILABLE)?;
        let live = owners
            .get(owner)
            .filter(|slot| slot.id == id)
            .and_then(|slot| slot.live.as_ref())
            .ok_or(UNAVAILABLE)?;
        live.input.try_send(input).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => {
                "SSH input queue is full; retry after pending input is sent".into()
            }
            mpsc::error::TrySendError::Closed(_) => UNAVAILABLE.into(),
        })
    }
    pub fn write(&self, owner: &str, id: &str, bytes: Vec<u8>) -> Result<(), String> {
        if bytes.is_empty() || bytes.len() > MAX_DATA {
            return Err("SSH input must contain between 1 and 32768 bytes".into());
        }
        self.input(owner, id, Input::Data(bytes))
    }
    pub fn resize(&self, owner: &str, id: &str, cols: u32, rows: u32) -> Result<(), String> {
        dimensions(cols, rows)?;
        self.input(owner, id, Input::Resize(cols, rows))
    }
    pub async fn read(&self, owner: &str, id: &str) -> Result<Option<Event>, String> {
        let output = {
            let owners = self.owners.lock().map_err(|_| UNAVAILABLE)?;
            owners
                .get(owner)
                .filter(|slot| slot.id == id)
                .and_then(|slot| slot.live.as_ref())
                .ok_or(UNAVAILABLE)?
                .output
                .clone()
        };
        // One outstanding read per session; no unbounded queue of IPC waiters.
        let mut output = output
            .try_lock()
            .map_err(|_| "SSH output read already pending")?;
        match tokio::time::timeout(Duration::from_millis(250), output.receiver.recv()).await {
            Ok(Some(event)) => Ok(Some(event)),
            Ok(None) if output.failed.swap(false, Ordering::SeqCst) => Ok(Some(Event::Failed)),
            Ok(None) => Ok(Some(Event::Closed)),
            Err(_) => Ok(None),
        }
    }
}
fn dimensions(cols: u32, rows: u32) -> Result<(), String> {
    if (1..=500).contains(&cols) && (1..=500).contains(&rows) {
        Ok(())
    } else {
        Err("SSH terminal dimensions must be between 1 and 500".into())
    }
}
async fn accepted(channel: &mut Channel<client::Msg>) -> Result<(), String> {
    match channel.wait().await {
        Some(ChannelMsg::Success) => Ok(()),
        _ => Err("SSH server refused or interrupted the PTY/shell request".into()),
    }
}
async fn run(
    connection: Authenticated,
    channel: Channel<client::Msg>,
    mut input: mpsc::Receiver<Input>,
    output: mpsc::Sender<Event>,
    failed: Arc<AtomicBool>,
) {
    // Retain the owned socket for the entire actor, including blocked output.
    let _shutdown = connection.shutdown;
    let _handle = connection.handle;
    let (mut reader, writer) = channel.split();
    let mut pending = None;
    let mut stall_deadline = tokio::time::Instant::now() + OUTPUT_STALL;
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(stall_deadline), if pending.is_some() => {
                failed.store(true, Ordering::SeqCst);
                break;
            },
            command = input.recv() => {
                let operation = async {
                    match command {
                        Some(Input::Data(bytes)) => writer.data(&bytes[..]).await,
                        Some(Input::Resize(cols, rows)) => writer.window_change(cols, rows, 0, 0).await,
                        None => Err(russh::Error::Disconnect),
                    }
                };
                let write_deadline = tokio::time::Instant::now() + Duration::from_secs(3);
                let deadline = if pending.is_some() { write_deadline.min(stall_deadline) } else { write_deadline };
                if !matches!(tokio::time::timeout_at(deadline, operation).await, Ok(Ok(()))) {
                    failed.store(true, Ordering::SeqCst);
                    break;
                }
            },
            message = reader.wait(), if pending.is_none() => {
                pending = match message {
                    Some(ChannelMsg::Data { data }) if data.len() <= MAX_DATA => Some(Event::Data { bytes: data.to_vec() }),
                    Some(ChannelMsg::ExtendedData { data, ext: 1 }) if data.len() <= MAX_DATA => Some(Event::Stderr { bytes: data.to_vec() }),
                    Some(ChannelMsg::ExitStatus { exit_status }) => Some(Event::Exit { status: exit_status }),
                    Some(ChannelMsg::Eof) => Some(Event::Eof),
                    Some(ChannelMsg::Close) | None => break,
                    Some(ChannelMsg::Data { .. } | ChannelMsg::ExtendedData { .. }) => { failed.store(true, Ordering::SeqCst); break; },
                    _ => None,
                };
                if pending.is_some() { stall_deadline = tokio::time::Instant::now() + OUTPUT_STALL; }
            },
            permit = output.reserve(), if pending.is_some() => {
                let Ok(permit) = permit else { break; };
                if let Some(event) = pending.take() { permit.send(event); }
            },
        }
    }
    // Dropping the sender lets the reader drain queued events then observe Closed.
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manager() -> (tempfile::TempDir, SessionManager) {
        let root = tempfile::tempdir().unwrap();
        let hosts = crate::ssh::HostManager::new(root.path().join("unused"));
        (root, SessionManager::new(AgentManager::new(hosts, None)))
    }
    #[tokio::test]
    async fn bounded_input_single_reader_and_owner_cleanup() {
        let (_root, manager) = manager();
        let (input, _commands) = mpsc::channel(CAPACITY);
        let (events, output) = mpsc::channel(CAPACITY);
        let output = Arc::new(AsyncMutex::new(Output {
            receiver: output,
            failed: Arc::new(AtomicBool::new(false)),
        }));
        let task = tokio::spawn(std::future::pending());
        let abort = task.abort_handle();
        manager.owners.lock().unwrap().insert(
            "main".into(),
            Slot {
                id: "owned".into(),
                selection: "selection".into(),
                live: Some(Live {
                    input,
                    output: output.clone(),
                    task,
                }),
            },
        );
        for _ in 0..CAPACITY {
            manager.write("main", "owned", vec![3]).unwrap();
        }
        assert_eq!(
            manager.write("main", "owned", vec![4]).err().unwrap(),
            "SSH input queue is full; retry after pending input is sent"
        );
        let held = output.lock().await;
        assert_eq!(
            manager.read("main", "owned").await.err().unwrap(),
            "SSH output read already pending"
        );
        drop(held);
        assert!(manager.read("main", "owned").await.unwrap().is_none());
        events
            .send(Event::Data {
                bytes: vec![0, 255, 3],
            })
            .await
            .unwrap();
        assert!(
            matches!(manager.read("main", "owned").await.unwrap(), Some(Event::Data { bytes }) if bytes == vec![0, 255, 3])
        );
        manager.forget_owner("main");
        tokio::task::yield_now().await;
        assert!(abort.is_finished());
        assert!(manager.read("main", "owned").await.is_err());
        assert!(!manager.owners.lock().unwrap().contains_key("main"));
    }
    #[tokio::test]
    async fn invalid_dimensions_and_capacity_fail_without_connecting_or_touching_trust() {
        let (root, manager) = manager();
        for (cols, rows) in [(0, 24), (80, 0), (501, 24), (80, u32::MAX)] {
            assert_eq!(
                manager
                    .start("main", "unused", "unused", cols, rows)
                    .await
                    .err()
                    .unwrap(),
                "SSH terminal dimensions must be between 1 and 500"
            );
        }
        for n in 0..CAPACITY {
            manager.owners.lock().unwrap().insert(
                format!("owner-{n}"),
                Slot {
                    id: "old".into(),
                    selection: "unused".into(),
                    live: None,
                },
            );
        }
        assert_eq!(
            manager
                .start("overflow", "unused", "unused", 80, 24)
                .await
                .err()
                .unwrap(),
            "Too many SSH sessions"
        );
        assert!(manager.close("owner-0", "stale").is_err());
        assert!(manager.current("owner-0", "old").is_ok());
        manager.close("owner-0", "old").unwrap();
        assert!(manager.current("owner-0", "old").is_err());
        let permits = manager.starts.acquire_many(CAPACITY as u32).await.unwrap();
        assert_eq!(
            manager
                .start("overflow", "unused", "unused", 80, 24)
                .await
                .err()
                .unwrap(),
            "Too many SSH session starts"
        );
        drop(permits);
        assert!(!root.path().join("unused").exists());
    }
}
