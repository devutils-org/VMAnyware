use super::*;
use crate::{fixtures::*, worker::Snapshot};

fn app() -> App {
    App::new(
        "https://localhost".into(),
        String::new(),
        Zeroizing::new(String::new()),
    )
}
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn snapshot(environments: Vec<Environment>, operations: Vec<Operation>) -> Box<Snapshot> {
    Box::new(Snapshot {
        environments: Ok(environments),
        operations: Ok(operations),
        detail: None,
        power: None,
        progress: None,
    })
}
fn loaded() -> App {
    let mut app = app();
    app.connected = true;
    app.apply(Reply::Refreshed(
        0,
        Selection::default(),
        snapshot(
            vec![environment(&"e".repeat(32)), environment(&"f".repeat(32))],
            vec![operation(&"d".repeat(32), &"e".repeat(32))],
        ),
    ));
    app
}

#[test]
fn navigation_search_cancel_and_selection_survive_reordering() {
    let mut app = loaded();
    let (send, _) = std::sync::mpsc::sync_channel(8);
    app.key(key(KeyCode::Down), &send);
    assert_eq!(app.environment_id, Some("f".repeat(32)));
    let mut es = app.environments.clone();
    es.reverse();
    app.apply(Reply::Refreshed(
        0,
        Selection::default(),
        snapshot(es, app.operations.clone()),
    ));
    assert_eq!(app.environment_id, Some("f".repeat(32)));
    assert_eq!(app.machines_state.selected(), Some(0));
    app.key(key(KeyCode::Char('/')), &send);
    app.paste("eeee");
    assert_eq!(app.machine_indices().len(), 1);
    assert_eq!(app.environment_id, Some("e".repeat(32)));
    app.key(key(KeyCode::Esc), &send);
    assert!(!app.searching);
    assert!(app.search.is_empty());
    assert_eq!(app.machine_indices().len(), 2);
    app.key(key(KeyCode::Char('3')), &send);
    assert!(app.view == View::Activity);
    assert_eq!(app.activity_filter, app.environment_id);
    app.key(key(KeyCode::Char('a')), &send);
    assert!(app.activity_filter.is_none());
}

#[test]
fn stale_selected_detail_and_old_connection_replies_are_discarded() {
    let mut app = loaded();
    let (send, _) = std::sync::mpsc::sync_channel(8);
    let old = app.environment_id.clone();
    app.key(key(KeyCode::Down), &send);
    let mut reply = snapshot(app.environments.clone(), app.operations.clone());
    let mut d = environment_value(old.as_ref().unwrap());
    d["definitionYaml"] = YAML.into();
    reply.detail = Some(Ok(serde_json::from_value(d).unwrap()));
    app.apply(Reply::Refreshed(
        0,
        Selection {
            environment: old,
            operation: None,
        },
        reply,
    ));
    assert!(app.detail.is_none());
    app.busy = false;
    app.key(key(KeyCode::Char('c')), &send);
    assert!(!app.connected);
    app.apply(Reply::Refreshed(
        0,
        Selection::default(),
        snapshot(vec![environment(&"e".repeat(32))], vec![]),
    ));
    assert!(app.environments.is_empty());
    assert!(!app.connected);
}

#[test]
fn errors_mark_cached_lists_stale_and_never_keep_old_power_as_current() {
    let mut app = loaded();
    let selection = app.selection();
    app.power = Some(Err("old power error".into()));
    app.apply(Reply::Refreshed(
        0,
        selection,
        Box::new(Snapshot {
            environments: Err("network lost".into()),
            operations: Err("network lost".into()),
            detail: None,
            power: None,
            progress: None,
        }),
    ));
    assert_eq!(app.environments.len(), 2);
    assert_eq!(app.operations.len(), 1);
    assert_eq!(app.environments_error.as_deref(), Some("network lost"));
    assert!(app.power.as_ref().unwrap().is_err());
}

#[test]
fn busy_network_does_not_block_quit_or_change_selected_machine() {
    let mut app = loaded();
    app.busy = true;
    let (send, _) = std::sync::mpsc::sync_channel(1);
    app.key(key(KeyCode::Down), &send);
    assert_eq!(app.environment_id, Some("f".repeat(32)));
    app.key(key(KeyCode::Char('q')), &send);
    assert!(app.quit);
    let mut login = super::App::new(
        "https://localhost".into(),
        String::new(),
        Zeroizing::new(String::new()),
    );
    login.busy = true;
    login.key(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &send,
    );
    assert!(login.quit);
}

#[test]
fn token_is_moved_out_of_form_on_connect_and_never_persisted() {
    let mut app = app();
    app.login
        .token
        .push_str("fixture-only-access-token-0123456789abcdef");
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    app.key(key(KeyCode::Enter), &send);
    assert!(app.login.token.is_empty());
    assert!(app.busy);
    match receive.try_recv().unwrap() {
        Request::Connect(_, c) => assert_eq!(
            c.token.as_str(),
            "fixture-only-access-token-0123456789abcdef"
        ),
        _ => panic!("expected connection"),
    }
}
