use crate::{
    api::Read,
    model::*,
    worker::{Connection, Reply, Request, Selection},
};
use ratatui::{
    crossterm::event::{
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    layout::{Position, Rect},
    widgets::ListState,
};
use std::{
    sync::mpsc::SyncSender,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Machines,
    Activity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    Key(KeyCode),
    Field(usize),
    Machine(usize),
    Operation(usize),
    Pane(bool),
}
pub struct Login {
    pub endpoint: String,
    pub ca_path: String,
    pub token: Zeroizing<String>,
    pub field: usize,
}
pub struct App {
    pub hits: Vec<(Rect, Click)>,
    pub login: Login,
    pub connected: bool,
    pub busy: bool,
    pub message: String,
    pub view: View,
    pub environments: Vec<Environment>,
    pub operations: Vec<Operation>,
    pub environments_error: Option<String>,
    pub operations_error: Option<String>,
    pub detail: Option<Read<Detail>>,
    pub power: Option<Read<Power>>,
    pub progress: Option<Read<Progress>>,
    pub machines_state: ListState,
    pub operations_state: ListState,
    pub environment_id: Option<String>,
    pub operation_id: Option<String>,
    pub activity_filter: Option<String>,
    pub definition: bool,
    pub detail_focus: bool,
    pub scroll: u16,
    pub search: String,
    pub searching: bool,
    search_before: String,
    pub help: bool,
    pub quit: bool,
    pub last_refresh: Option<Instant>,
    epoch: u64,
    next_refresh: Instant,
    refresh_requested: bool,
}
impl App {
    pub fn new(endpoint: String, ca_path: String, token: Zeroizing<String>) -> Self {
        Self {
            hits: Vec::new(),
            login: Login {
                endpoint,
                ca_path,
                field: if token.is_empty() { 2 } else { 0 },
                token,
            },
            connected: false,
            busy: false,
            message: String::new(),
            view: View::Machines,
            environments: vec![],
            operations: vec![],
            environments_error: None,
            operations_error: None,
            detail: None,
            power: None,
            progress: None,
            machines_state: ListState::default(),
            operations_state: ListState::default(),
            environment_id: None,
            operation_id: None,
            activity_filter: None,
            definition: false,
            detail_focus: false,
            scroll: 0,
            search: String::new(),
            searching: false,
            search_before: String::new(),
            help: false,
            quit: false,
            last_refresh: None,
            epoch: 0,
            next_refresh: Instant::now(),
            refresh_requested: false,
        }
    }
    pub fn machine_indices(&self) -> Vec<usize> {
        let query = self.search.to_lowercase();
        self.environments
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                format!(
                    "{} {} {}",
                    e.definition_name, e.provider_reference.node, e.provider_reference.vmid
                )
                .to_lowercase()
                .contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn operation_indices(&self) -> Vec<usize> {
        let query = self.search.to_lowercase();
        self.operations
            .iter()
            .enumerate()
            .filter(|(_, op)| {
                self.activity_filter
                    .as_ref()
                    .is_none_or(|id| op.instance_id.as_ref() == Some(id))
                    && format!(
                        "{} {} {} {}",
                        op.kind,
                        op.status.label(),
                        op.id,
                        op.instance_id.as_deref().unwrap_or("")
                    )
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn environment(&self) -> Option<&Environment> {
        self.environments
            .iter()
            .find(|e| Some(&e.id) == self.environment_id.as_ref())
    }
    pub fn operation(&self) -> Option<&Operation> {
        self.operations
            .iter()
            .find(|op| Some(&op.id) == self.operation_id.as_ref())
    }

    /// Preserve selections by identity when history is reordered by a refresh.
    fn reconcile(&mut self) {
        let indices = self.machine_indices();
        let selected = indices
            .iter()
            .position(|&i| Some(&self.environments[i].id) == self.environment_id.as_ref())
            .or(if indices.is_empty() { None } else { Some(0) });
        self.machines_state.select(selected);
        let environment_id = selected.map(|i| self.environments[indices[i]].id.clone());
        if environment_id != self.environment_id {
            self.environment_id = environment_id;
            self.detail = None;
            self.power = None;
            self.scroll = 0;
            self.refresh_requested = true;
        }
        let indices = self.operation_indices();
        let selected = indices
            .iter()
            .position(|&i| Some(&self.operations[i].id) == self.operation_id.as_ref())
            .or(if indices.is_empty() { None } else { Some(0) });
        self.operations_state.select(selected);
        let operation_id = selected.map(|i| self.operations[indices[i]].id.clone());
        if operation_id != self.operation_id {
            self.operation_id = operation_id;
            self.progress = None;
            self.refresh_requested = true;
        }
    }
    fn selection(&self) -> Selection {
        Selection {
            environment: if self.view == View::Machines {
                self.environment_id.clone()
            } else {
                None
            },
            operation: if self.view == View::Activity {
                self.operation_id.clone()
            } else {
                None
            },
        }
    }
    pub fn connect(&mut self, send: &SyncSender<Request>) {
        if self.busy {
            return;
        }
        self.epoch += 1;
        let connection = Connection {
            endpoint: self.login.endpoint.clone(),
            ca_path: self.login.ca_path.clone(),
            token: Zeroizing::new(std::mem::take(&mut *self.login.token)),
        };
        if send
            .try_send(Request::Connect(self.epoch, connection))
            .is_ok()
        {
            self.busy = true;
            self.message = "Connecting over verified HTTPS…".into();
        } else {
            self.message = "Network reader unavailable. Restart the TUI.".into();
        }
    }
    pub fn tick(&mut self, send: &SyncSender<Request>) {
        if self.connected
            && !self.busy
            && (self.refresh_requested || Instant::now() >= self.next_refresh)
        {
            if send
                .try_send(Request::Refresh(self.epoch, self.selection()))
                .is_ok()
            {
                self.busy = true;
                self.refresh_requested = false;
            } else {
                self.message = "Network reader unavailable. Restart the TUI.".into();
            }
        }
    }
    pub fn apply(&mut self, reply: Reply) {
        match reply {
            Reply::Connected(epoch, result) if epoch == self.epoch => {
                self.busy = false;
                match result {
                    Ok(()) => {
                        self.connected = true;
                        self.message.clear();
                        self.refresh_requested = true;
                    }
                    Err(error) => self.message = error,
                }
            }
            Reply::Refreshed(epoch, selection, snapshot)
                if epoch == self.epoch && self.connected =>
            {
                self.busy = false;
                self.next_refresh = Instant::now() + Duration::from_secs(5);
                self.last_refresh = Some(Instant::now());
                match snapshot.environments {
                    Ok(items) => {
                        self.environments = items;
                        self.environments_error = None;
                    }
                    Err(error) => self.environments_error = Some(error),
                }
                match snapshot.operations {
                    Ok(items) => {
                        self.operations = items;
                        self.operations_error = None;
                    }
                    Err(error) => self.operations_error = Some(error),
                }
                self.reconcile();
                // A selection may change while HTTPS is in flight. Never put an
                // old machine's data into the newly selected machine's panel.
                if selection.environment == self.environment_id && self.view == View::Machines {
                    let error = || {
                        "The selected environment could not be refreshed. Press r to retry."
                            .to_owned()
                    };
                    self.detail = snapshot
                        .detail
                        .or_else(|| self.environment_id.as_ref().map(|_| Err(error())));
                    self.power = snapshot
                        .power
                        .or_else(|| self.environment_id.as_ref().map(|_| Err(error())));
                }
                if selection.operation == self.operation_id && self.view == View::Activity {
                    self.progress = snapshot.progress.or_else(|| {
                        self.operation_id.as_ref().map(|_| {
                            Err(
                                "The selected operation could not be refreshed. Press r to retry."
                                    .into(),
                            )
                        })
                    });
                }
            }
            _ => (),
        }
    }
    fn move_selection(&mut self, delta: isize) {
        let (indices, current) = match self.view {
            View::Machines => (self.machine_indices(), self.machines_state.selected()),
            View::Activity => (self.operation_indices(), self.operations_state.selected()),
        };
        if indices.is_empty() {
            return;
        }
        let index = current
            .unwrap_or(0)
            .saturating_add_signed(delta)
            .min(indices.len() - 1);
        match self.view {
            View::Machines => {
                self.environment_id = Some(self.environments[indices[index]].id.clone());
                self.detail = None;
                self.power = None;
            }
            View::Activity => {
                self.operation_id = Some(self.operations[indices[index]].id.clone());
                self.progress = None;
            }
        }
        self.scroll = 0;
        self.refresh_requested = true;
        self.reconcile();
    }
    pub fn paste(&mut self, text: &str) {
        if !self.connected && !self.busy {
            let field = match self.login.field {
                0 => &mut self.login.endpoint,
                1 => &mut self.login.ca_path,
                _ => &mut *self.login.token,
            };
            if field.len() + text.len() <= 4096 && !text.chars().any(char::is_control) {
                field.push_str(text);
            }
        } else if self.searching
            && self.search.len() + text.len() <= 128
            && !text.chars().any(char::is_control)
        {
            self.search.push_str(text);
            self.reconcile();
        }
    }
    pub fn key(&mut self, key: KeyEvent, send: &SyncSender<Request>) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if !self.connected {
            if key.code == KeyCode::Esc {
                self.quit = true;
                return;
            }
            if self.busy {
                return;
            }
            match key.code {
                KeyCode::Tab | KeyCode::Down => self.login.field = (self.login.field + 1) % 3,
                KeyCode::BackTab | KeyCode::Up => self.login.field = (self.login.field + 2) % 3,
                KeyCode::Enter => self.connect(send),
                KeyCode::Backspace => match self.login.field {
                    0 => {
                        self.login.endpoint.pop();
                    }
                    1 => {
                        self.login.ca_path.pop();
                    }
                    _ => {
                        self.login.token.pop();
                    }
                },
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    match self.login.field {
                        0 => self.login.endpoint.clear(),
                        1 => self.login.ca_path.clear(),
                        _ => self.login.token.clear(),
                    }
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.paste(&c.to_string())
                }
                _ => (),
            }
            return;
        }
        if self.help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter) {
                self.help = false;
            }
            return;
        }
        if self.searching {
            match key.code {
                KeyCode::Esc => {
                    self.search = self.search_before.clone();
                    self.searching = false;
                    self.reconcile();
                }
                KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.search.pop();
                    self.reconcile();
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.paste(&c.to_string())
                }
                _ => (),
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') => self.help = true,
            KeyCode::Char('r') => self.refresh_requested = true,
            KeyCode::Char('c') if !self.busy => {
                self.epoch += 1;
                self.connected = false;
                self.environments.clear();
                self.operations.clear();
                self.detail = None;
                self.power = None;
                self.progress = None;
                self.environment_id = None;
                self.operation_id = None;
                self.activity_filter = None;
                self.message.clear();
                self.login.field = 2;
                let _ = send.try_send(Request::Disconnect);
            }
            KeyCode::Char('/') => {
                self.search_before = self.search.clone();
                self.searching = true;
            }
            KeyCode::Esc => {
                self.search.clear();
                self.detail_focus = false;
                self.scroll = 0;
                self.reconcile();
            }
            KeyCode::Char('m') => self.change_view(View::Machines, None),
            KeyCode::Char('a') => self.change_view(View::Activity, None),
            KeyCode::Char('3') if self.view == View::Machines => {
                self.change_view(View::Activity, self.environment_id.clone())
            }
            KeyCode::Char('1') if self.view == View::Machines => {
                self.definition = false;
                self.scroll = 0;
            }
            KeyCode::Char('2') if self.view == View::Machines => {
                self.definition = true;
                self.detail_focus = true;
                self.scroll = 0;
            }
            KeyCode::Tab | KeyCode::BackTab => self.detail_focus = !self.detail_focus,
            KeyCode::Enter | KeyCode::Right => {
                self.detail_focus = true;
                self.refresh_requested = true;
            }
            KeyCode::Left => self.detail_focus = false,
            KeyCode::Down | KeyCode::Char('j') => {
                if self.detail_focus {
                    self.scroll = self.scroll.saturating_add(1);
                } else {
                    self.move_selection(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.detail_focus {
                    self.scroll = self.scroll.saturating_sub(1);
                } else {
                    self.move_selection(-1);
                }
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Home => self.scroll = 0,
            _ => (),
        }
    }
    pub fn mouse(&mut self, event: MouseEvent, send: &SyncSender<Request>) {
        let action = self
            .hits
            .iter()
            .rev()
            .find(|(area, _)| area.contains(Position::new(event.column, event.row)))
            .map(|(_, action)| *action);
        if matches!(
            event.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ) {
            if !self.connected || self.help || self.searching {
                return;
            }
            let detail = match action {
                Some(Click::Pane(detail)) => detail,
                Some(Click::Machine(_) | Click::Operation(_)) => false,
                _ => return,
            };
            self.detail_focus = detail;
            let delta = if event.kind == MouseEventKind::ScrollDown {
                3
            } else {
                -3
            };
            if detail {
                self.scroll = self.scroll.saturating_add_signed(delta as i16);
            } else {
                self.move_selection(delta);
            }
            return;
        }
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }
        if self.help {
            if action == Some(Click::Key(KeyCode::Esc)) {
                self.help = false;
                self.hits.clear();
            }
            return;
        }
        let Some(action) = action else {
            return;
        };
        // Clicking a command finishes search without inserting its shortcut in
        // the query. The command still follows the existing keyboard guards.
        if self.searching && !matches!(action, Click::Key(KeyCode::Char('/'))) {
            self.searching = false;
        }
        match action {
            Click::Key(key) => self.key(KeyEvent::new(key, KeyModifiers::NONE), send),
            Click::Field(field) if !self.connected && !self.busy => self.login.field = field,
            Click::Machine(index) if self.connected => {
                if let Some(e) = self.environments.get(index) {
                    self.environment_id = Some(e.id.clone());
                    self.detail = None;
                    self.power = None;
                    self.scroll = 0;
                    self.detail_focus = true;
                    self.refresh_requested = true;
                    self.reconcile();
                }
            }
            Click::Operation(index) if self.connected => {
                if let Some(op) = self.operations.get(index) {
                    self.operation_id = Some(op.id.clone());
                    self.progress = None;
                    self.scroll = 0;
                    self.detail_focus = true;
                    self.refresh_requested = true;
                    self.reconcile();
                }
            }
            Click::Pane(detail) if self.connected => self.detail_focus = detail,
            _ => (),
        }
        // Geometry is rebuilt by the next draw, including modal and narrow-pane changes.
        self.hits.clear();
    }
    fn change_view(&mut self, view: View, filter: Option<String>) {
        self.view = view;
        self.activity_filter = filter;
        self.search.clear();
        self.scroll = 0;
        self.detail_focus = false;
        self.refresh_requested = true;
        self.reconcile();
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
