//! Actual local OS shell exercise, using the shared disposable PTY fixture.
use super::*;
pub(super) use shell::Shell;
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
            {
                let shells = fixture.shells.lock().unwrap();
                let shell = shells.last().unwrap().lock().unwrap();
                assert!(shell.process_id().is_some());
                assert!(!shell.is_closed());
            }
            channel
                // Produce our own line boundary; shell command echo differs by OS.
                .data(&b"stty -echo; printf '\\nREADY\\n'\n"[..])
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
                        shell.foreground_child().is_some()
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
