//! Owned localhost SSH/OS-shell fixture for actual Android native checks.
#[cfg(unix)]
#[path = "../src/ssh/agent/fixture.rs"]
#[allow(dead_code)]
mod fixture;

#[cfg(unix)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    use russh::keys::{HashAlg, PublicKey};
    use serde_json::{json, Value};
    use std::{
        io::{BufRead, Write},
        sync::atomic::Ordering,
        time::Duration,
    };
    assert_eq!(
        std::env::args().nth(1).as_deref(),
        Some("--owned-android-fixture")
    );
    // Read one public key over the owned pipe. Never read a private key/config.
    let mut first = String::new();
    std::io::stdin().read_line(&mut first).unwrap();
    assert!(first.len() <= 8192);
    let request: Value = serde_json::from_str(&first).unwrap();
    let key = PublicKey::from_openssh(request["publicKey"].as_str().unwrap()).unwrap();
    let fixture =
        fixture::Fixture::with_timeout(key, true, None, true, Duration::from_secs(90)).await;
    let fingerprint = || {
        fixture
            .host_key
            .lock()
            .unwrap()
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string()
    };
    println!(
        "{}",
        json!({"ready":true,"host":"10.0.2.2","port":fixture.port,"fingerprint":fingerprint()})
    );
    std::io::stdout().flush().unwrap();
    let (sender, mut commands) = tokio::sync::mpsc::channel(8);
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if line.len() > 8192 || sender.blocking_send(line).is_err() {
                break;
            }
        }
    });
    while let Some(line) = commands.recv().await {
        let request: Value = serde_json::from_str(&line).unwrap();
        let action = request["action"].as_str().unwrap();
        if action == "rotate" {
            *fixture.host_key.lock().unwrap() = fixture::key();
        }
        if action == "stop" {
            break;
        }
        assert!(matches!(action, "state" | "rotate"));
        let mut ptys = vec![];
        let mut sizes = vec![];
        let mut bytes = 0;
        for event in fixture.events.lock().unwrap().iter() {
            match event {
                fixture::Protocol::Pty(cols, rows) => ptys.push([*cols, *rows]),
                fixture::Protocol::Resize(cols, rows) => sizes.push([*cols, *rows]),
                fixture::Protocol::Data(data) => bytes += data.len(),
                fixture::Protocol::Shell => {}
            }
        }
        let shells = fixture.shells.lock().unwrap();
        let foreground = shells
            .iter()
            .any(|shell| shell.lock().unwrap().foreground_child().is_some());
        let closed = shells.iter().all(|shell| shell.lock().unwrap().is_closed());
        println!(
            "{}",
            json!({"fingerprint":fingerprint(),"authAttempts":fixture.auth.load(Ordering::SeqCst),"channels":fixture.channels.load(Ordering::SeqCst),"connections":fixture.connections.load(Ordering::SeqCst),"ptys":ptys,"sizes":sizes,"inputBytes":bytes,"foreground":foreground,"shellsClosed":closed})
        );
        std::io::stdout().flush().unwrap();
    }
    fixture.shutdown();
}
#[cfg(not(unix))]
fn main() {
    panic!("This owned OS-shell fixture requires Unix");
}
