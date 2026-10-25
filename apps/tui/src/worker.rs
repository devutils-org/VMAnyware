//! One bounded background reader keeps the keyboard responsive during timeouts.
use crate::{
    api::{Api, Read},
    model::*,
};
use std::{
    io::Read as _,
    sync::mpsc::{self, Receiver, SyncSender},
};
use zeroize::Zeroizing;

pub struct Connection {
    pub endpoint: String,
    pub token: Zeroizing<String>,
    pub ca_path: String,
}
pub enum Request {
    Connect(u64, Connection),
    Refresh(u64, Selection),
    Disconnect,
}
#[derive(Clone, Default)]
pub struct Selection {
    pub environment: Option<String>,
    pub operation: Option<String>,
}
pub struct Snapshot {
    pub environments: Read<Vec<Environment>>,
    pub operations: Read<Vec<Operation>>,
    pub detail: Option<Read<Detail>>,
    pub power: Option<Read<Power>>,
    pub progress: Option<Read<Progress>>,
}
pub enum Reply {
    Connected(u64, Read<()>),
    Refreshed(u64, Selection, Box<Snapshot>),
}

pub fn spawn() -> (SyncSender<Request>, Receiver<Reply>) {
    let (send, requests) = mpsc::sync_channel(1);
    let (replies, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let mut api = None;
        while let Ok(request) = requests.recv() {
            match request {
                Request::Connect(epoch, connection) => {
                    api = None;
                    let result = (|| {
                        let runtime = runtime
                            .as_ref()
                            .map_err(|_| "Cannot start the network reader.")?;
                        let ca = if connection.ca_path.is_empty() {
                            None
                        } else {
                            let file = std::fs::File::open(&connection.ca_path)
                                .map_err(|_| "Cannot open the public CA file.")?;
                            let mut bytes = Vec::new();
                            file.take(16 * 1024 + 1)
                                .read_to_end(&mut bytes)
                                .map_err(|_| "Cannot read the public CA file.")?;
                            Some(bytes)
                        };
                        let client =
                            Api::new(&connection.endpoint, &connection.token, ca.as_deref())?;
                        runtime.block_on(client.check())?;
                        api = Some(client);
                        Ok(())
                    })();
                    if replies.send(Reply::Connected(epoch, result)).is_err() {
                        break;
                    }
                }
                Request::Refresh(epoch, selection) => {
                    if let (Some(api), Ok(runtime)) = (&api, &runtime) {
                        let snapshot = runtime.block_on(refresh(api, &selection));
                        if replies
                            .send(Reply::Refreshed(epoch, selection, Box::new(snapshot)))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                Request::Disconnect => api = None,
            }
        }
    });
    (send, receive)
}

async fn refresh(api: &Api, selection: &Selection) -> Snapshot {
    let (environments, operations) = tokio::join!(api.environments(), api.operations());
    let environment = environments.as_ref().ok().and_then(|es| {
        es.iter()
            .find(|e| Some(&e.id) == selection.environment.as_ref())
    });
    let operation = operations.as_ref().ok().and_then(|ops| {
        ops.iter()
            .find(|op| Some(&op.id) == selection.operation.as_ref())
    });
    let detail = async {
        match environment {
            Some(e) => Some(api.detail(e).await),
            None => None,
        }
    };
    let power = async {
        match environment {
            Some(e) if e.status == Association::Registered => Some(api.power(e).await),
            _ => None,
        }
    };
    let progress = async {
        match (operation, environments.as_ref()) {
            (Some(op), Ok(es)) if op.provisioning() => Some(api.progress(op, es).await),
            _ => None,
        }
    };
    let (detail, power, progress) = tokio::join!(detail, power, progress);
    Snapshot {
        environments,
        operations,
        detail,
        power,
        progress,
    }
}
