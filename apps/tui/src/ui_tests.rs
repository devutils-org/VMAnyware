use super::*;
use crate::app::Click;
use crate::{
    fixtures::*,
    worker::{Reply, Selection, Snapshot},
};
use ratatui::crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::TestBackend, Terminal};
use zeroize::Zeroizing;

fn mouse(app: &mut App, action: Click, kind: MouseEventKind) {
    let area = app
        .hits
        .iter()
        .rev()
        .find(|(_, a)| *a == action)
        .expect("visible click target")
        .0;
    let (send, _receive) = std::sync::mpsc::sync_channel(4);
    app.mouse(
        MouseEvent {
            kind,
            column: area.x,
            row: area.y + u16::from(matches!(action, Click::Pane(true))),
            modifiers: KeyModifiers::NONE,
        },
        &send,
    );
}

fn click(app: &mut App, action: Click) {
    mouse(app, action, MouseEventKind::Down(MouseButton::Left));
}

#[test]
fn mouse_uses_rendered_rows_fields_tabs_and_modal_targets() {
    let mut app = App::new(
        "https://localhost".into(),
        String::new(),
        Zeroizing::new(String::new()),
    );
    screen(&mut app, 120, 35);
    click(&mut app, Click::Field(1));
    assert_eq!(app.login.field, 1);
    app.connected = true;
    app.environments = vec![environment(&"e".repeat(32)), environment(&"f".repeat(32))];
    app.environment_id = Some("e".repeat(32));
    app.operations = vec![operation(&"d".repeat(32), &"f".repeat(32))];
    screen(&mut app, 120, 35);
    click(&mut app, Click::Machine(1));
    assert_eq!(app.environment_id, Some("f".repeat(32)));
    assert!(app.detail_focus);
    screen(&mut app, 120, 35);
    click(&mut app, Click::Key(KeyCode::Char('2')));
    assert!(app.definition);
    screen(&mut app, 120, 35);
    click(&mut app, Click::Key(KeyCode::Char('3')));
    assert!(app.view == View::Activity);
    assert_eq!(app.activity_filter, Some("f".repeat(32)));
    screen(&mut app, 120, 35);
    click(&mut app, Click::Operation(0));
    assert_eq!(app.operation_id, Some("d".repeat(32)));
    screen(&mut app, 120, 35);
    click(&mut app, Click::Key(KeyCode::Char('?')));
    screen(&mut app, 120, 35);
    assert_eq!(app.hits.len(), 1); // Overlay blocks every underlying command.
    click(&mut app, Click::Key(KeyCode::Esc));
    assert!(!app.help);
    screen(&mut app, 20, 8);
    assert!(app.hits.is_empty());
}

#[test]
fn mouse_respects_list_offset_filters_and_narrow_footer_wrapping() {
    let mut app = App::new(
        "https://localhost".into(),
        String::new(),
        Zeroizing::new(String::new()),
    );
    app.connected = true;
    app.environments = (1..=30)
        .map(|id| environment(&format!("{id:032x}")))
        .collect();
    app.machines_state.select(Some(24));
    app.environment_id = Some(app.environments[24].id.clone());
    screen(&mut app, 120, 20);
    let offset = app.machines_state.offset();
    assert!(offset > 0);
    click(&mut app, Click::Machine(offset));
    assert_eq!(
        app.environment_id,
        Some(app.environments[offset].id.clone())
    );
    app.search = "no-match".into();
    screen(&mut app, 120, 20);
    assert!(!app.hits.iter().any(|(_, a)| matches!(a, Click::Machine(_))));
    app.search.clear();
    app.detail_focus = false;
    screen(&mut app, 40, 24);
    assert!(app
        .hits
        .iter()
        .any(|(_, a)| *a == Click::Key(KeyCode::Char('q'))));
    let visible = app
        .hits
        .iter()
        .find_map(|(_, a)| {
            if matches!(a, Click::Machine(_)) {
                Some(*a)
            } else {
                None
            }
        })
        .unwrap();
    mouse(&mut app, visible, MouseEventKind::ScrollDown);
    assert!(!app.detail_focus);
    screen(&mut app, 40, 24);
    click(&mut app, Click::Key(KeyCode::Tab));
    screen(&mut app, 40, 24);
    assert!(!app.hits.iter().any(|(_, a)| matches!(a, Click::Machine(_))));
    mouse(&mut app, Click::Pane(true), MouseEventKind::ScrollDown);
    assert_eq!(app.scroll, 3);
}

fn screen(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>()
}
#[test]
fn login_masks_token_and_small_and_narrow_screens_render() {
    let mut app = App::new(
        "https://localhost".into(),
        String::new(),
        Zeroizing::new("never-display-this-token".into()),
    );
    let text = screen(&mut app, 100, 30);
    assert!(text.contains("Access token"));
    assert!(!text.contains("never-display-this-token"));
    assert!(screen(&mut app, 20, 8).contains("Resize"));
    assert!(screen(&mut app, 45, 24).contains("connect"));
}

#[test]
fn machine_and_activity_panels_keep_registration_and_uncertainty_explicit() {
    let eid = "e".repeat(32);
    let oid = "d".repeat(32);
    let mut app = App::new(
        "https://localhost".into(),
        String::new(),
        Zeroizing::new(String::new()),
    );
    app.connected = true;
    let mut op = operation(&oid, &eid);
    op.status = Status::Uncertain;
    app.apply(Reply::Refreshed(
        0,
        Selection::default(),
        Box::new(Snapshot {
            environments: Ok(vec![environment(&eid)]),
            operations: Ok(vec![op]),
            detail: None,
            power: None,
            progress: None,
        }),
    ));
    let text = screen(&mut app, 120, 35);
    assert!(text.contains("registered"));
    assert!(text.contains("Historical connection hint"));
    app.operations_error = Some("History unavailable".into());
    assert!(screen(&mut app, 120, 35).contains("STALE"));
    app.operations_error = None;
    app.view = View::Activity;
    let text = screen(&mut app, 120, 35);
    assert!(text.contains("uncertain"));
    assert!(text.contains("Outcome unconfirmed"));
    app.detail_focus = true;
    assert!(screen(&mut app, 55, 35).contains("Recorded operation"));
    app.help = true;
    assert!(screen(&mut app, 90, 30).contains("Keyboard help"));
}

#[test]
fn server_text_cannot_inject_terminal_or_bidi_controls() {
    let text = display("hello\u{1b}[2J\r\u{202e}world\nnext");
    assert!(!text.contains('\u{1b}') && !text.contains('\r') && !text.contains('\u{202e}'));
    assert!(text.contains('\n'));
}
