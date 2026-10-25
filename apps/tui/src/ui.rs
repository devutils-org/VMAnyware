use crate::{
    app::{App, Click, View},
    model::{display, Association, Status},
};
use ratatui::{
    crossterm::event::KeyCode,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

const ACCENT: Color = Color::Cyan;
fn block(title: impl Into<Line<'static>>, active: bool) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(if active { ACCENT } else { Color::DarkGray }))
}
fn status(value: Status) -> Span<'static> {
    Span::styled(
        value.label(),
        Style::default().fg(match value {
            Status::Succeeded => Color::Green,
            Status::Failed => Color::Red,
            Status::Uncertain | Status::Interrupted => Color::Yellow,
            Status::Running => ACCENT,
            Status::Queued => Color::Gray,
        }),
    )
}
fn muted(text: impl Into<String>) -> Line<'static> {
    Line::styled(text.into(), Style::default().fg(Color::Gray))
}
fn value(label: &str, text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(Color::Gray)),
        Span::raw(display(text)),
    ])
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    app.hits.clear();
    let area = frame.area();
    if area.width < 40 || area.height < 12 {
        frame.render_widget(
            Paragraph::new("VMAnyware\nResize to at least 40 × 12.\nCtrl-C to quit."),
            area,
        );
        return;
    }
    if !app.connected {
        login(frame, app);
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                " VMAnyware ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            muted(format!(" {}", app.login.endpoint)),
        ]),
        rows[0],
    );
    buttons(
        frame,
        app,
        Rect::new(area.x + 12, rows[0].y, area.width.saturating_sub(12), 1),
        &[
            ("Machines", KeyCode::Char('m')),
            ("Activity", KeyCode::Char('a')),
        ],
    );

    // A narrow terminal shows the focused pane at full width; Tab switches it.
    let panes = if rows[1].width >= 90 {
        let split = Layout::horizontal([Constraint::Percentage(36), Constraint::Percentage(64)])
            .split(rows[1]);
        (Some(split[0]), Some(split[1]))
    } else if app.detail_focus {
        (None, Some(rows[1]))
    } else {
        (Some(rows[1]), None)
    };
    if let Some(area) = panes.0 {
        list(frame, app, area);
    }
    if let Some(area) = panes.1 {
        detail(frame, app, area);
    }
    let error = match app.view {
        View::Machines => app
            .environments_error
            .as_deref()
            .or(app.operations_error.as_deref()),
        View::Activity => app
            .operations_error
            .as_deref()
            .or(app.environments_error.as_deref()),
    };
    let feedback = if app.searching {
        format!(" /{}█  Enter apply · Esc cancel", app.search)
    } else if let Some(error) = error {
        format!(" STALE · {error}")
    } else if !app.message.is_empty() {
        app.message.clone()
    } else if app.busy {
        " Reading server…".into()
    } else if !app.search.is_empty() {
        format!(" Filter: {} · Esc clear", app.search)
    } else {
        format!(
            " Refresh every 5s · last read {}s ago",
            app.last_refresh.map(|t| t.elapsed().as_secs()).unwrap_or(0)
        )
    };
    frame.render_widget(
        Paragraph::new(vec![Line::styled(
            feedback,
            Style::default().fg(if error.is_some() {
                Color::Yellow
            } else {
                Color::Gray
            }),
        )]),
        rows[2],
    );
    buttons(
        frame,
        app,
        Rect::new(
            rows[2].x,
            rows[2].y + 1,
            rows[2].width,
            rows[2].height.saturating_sub(1),
        ),
        &[
            ("Pane", KeyCode::Tab),
            ("Search", KeyCode::Char('/')),
            ("Refresh", KeyCode::Char('r')),
            ("Connect", KeyCode::Char('c')),
            ("?", KeyCode::Char('?')),
            ("Quit", KeyCode::Char('q')),
        ],
    );
    if app.help {
        app.hits.clear();
        help(frame, app);
    }
}

fn login(frame: &mut Frame, app: &mut App) {
    let outer = centered(frame.area(), 76, 19);
    let panel = block(" VMAnyware · connect ", true);
    let inner = panel.inner(outer);
    frame.render_widget(panel, outer);
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new("Connect to your control plane over verified HTTPS."),
        rows[0],
    );
    let fields = [
        (" Server origin ", app.login.endpoint.as_str()),
        (" Public CA file · optional ", app.login.ca_path.as_str()),
        (
            " Access token · hidden ",
            if app.login.token.is_empty() {
                ""
            } else {
                "••••••••"
            },
        ),
    ];
    for (i, (title, text)) in fields.into_iter().enumerate() {
        app.hits.push((rows[i + 1], Click::Field(i)));
        frame.render_widget(
            Paragraph::new(display(text)).block(block(title, app.login.field == i)),
            rows[i + 1],
        );
    }
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                display(&app.message),
                Style::default().fg(if app.busy { ACCENT } else { Color::Yellow }),
            ),
            muted("Tab / Shift-Tab field · Ctrl-U clear · Enter connect · Esc quit"),
            muted("Token stays in session memory. No configuration is saved."),
        ])
        .wrap(Wrap { trim: false }),
        rows[4],
    );
    buttons(
        frame,
        app,
        rows[5],
        &[("Connect", KeyCode::Enter), ("Quit", KeyCode::Esc)],
    );
}

/// Render and hit-test the same visible button cells, wrapping on narrow screens.
fn buttons(frame: &mut Frame, app: &mut App, area: Rect, items: &[(&str, KeyCode)]) {
    let (mut x, mut y) = (area.x, area.y);
    for (label, key) in items {
        let text = format!("[{label}]");
        let width = text.len() as u16;
        if x + width > area.right() {
            x = area.x;
            y += 1;
        }
        if y >= area.bottom() || width > area.width {
            break;
        }
        let rect = Rect::new(x, y, width, 1);
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(ACCENT)),
            rect,
        );
        app.hits.push((rect, Click::Key(*key)));
        x += width + 1;
    }
}

fn list(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner = Block::bordered().inner(area);
    app.hits.push((inner, Click::Pane(false)));
    let active = !app.detail_focus;
    match app.view {
        View::Machines => {
            let indices = app.machine_indices();
            let items: Vec<ListItem> = indices
                .iter()
                .map(|&i| {
                    let e = &app.environments[i];
                    let marker = match e.status {
                        Association::Registered => Color::Gray,
                        Association::Deleted => Color::Yellow,
                        Association::Released => Color::DarkGray,
                    };
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::raw(format!("{} ", e.definition_name)),
                            Span::styled(e.status.label(), Style::default().fg(marker)),
                        ]),
                        muted(format!(
                            "{} · VM {}",
                            e.provider_reference.node, e.provider_reference.vmid
                        )),
                    ])
                })
                .collect();
            let title = format!(" Machines · {} ", indices.len());
            if items.is_empty() {
                frame.render_widget(
                    Paragraph::new(if app.environments_error.is_some() {
                        "Cannot load machines. Press r to retry."
                    } else if app.busy {
                        "Loading…"
                    } else {
                        "No matching environments."
                    })
                    .block(block(title, active)),
                    area,
                );
            } else {
                frame.render_stateful_widget(
                    List::new(items)
                        .block(block(title, active))
                        .highlight_symbol("› ")
                        .highlight_style(
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        ),
                    area,
                    &mut app.machines_state,
                );
                list_hits(app, inner, &indices, 2, app.machines_state.offset(), true);
            }
        }
        View::Activity => {
            let indices = app.operation_indices();
            let items: Vec<ListItem> = indices
                .iter()
                .map(|&i| {
                    let op = &app.operations[i];
                    let name = app
                        .environments
                        .iter()
                        .find(|e| Some(&e.id) == op.instance_id.as_ref())
                        .map(|e| e.definition_name.as_str())
                        .unwrap_or("control plane");
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::raw(format!("{} · ", op.kind.replace('_', " "))),
                            status(op.status),
                        ]),
                        muted(name),
                        muted(display(&op.updated_at)),
                    ])
                })
                .collect();
            let filter = if app.activity_filter.is_some() {
                "machine"
            } else {
                "all"
            };
            let title = format!(" Activity · {filter} · {} ", indices.len());
            if items.is_empty() {
                frame.render_widget(
                    Paragraph::new(if app.operations_error.is_some() {
                        "Cannot load history. Press r to retry."
                    } else if app.busy {
                        "Loading…"
                    } else {
                        "No matching operations in recent history."
                    })
                    .block(block(title, active)),
                    area,
                );
            } else {
                frame.render_stateful_widget(
                    List::new(items)
                        .block(block(title, active))
                        .highlight_symbol("› ")
                        .highlight_style(
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        ),
                    area,
                    &mut app.operations_state,
                );
                list_hits(
                    app,
                    inner,
                    &indices,
                    3,
                    app.operations_state.offset(),
                    false,
                );
            }
        }
    }
}

fn list_hits(
    app: &mut App,
    area: Rect,
    indices: &[usize],
    height: u16,
    offset: usize,
    machines: bool,
) {
    for (row, &index) in indices.iter().skip(offset).enumerate() {
        let y = area.y + row as u16 * height;
        if y >= area.bottom() {
            break;
        }
        app.hits.push((
            Rect::new(area.x, y, area.width, height.min(area.bottom() - y)),
            if machines {
                Click::Machine(index)
            } else {
                Click::Operation(index)
            },
        ));
    }
}

fn detail(frame: &mut Frame, app: &mut App, area: Rect) {
    let (title, lines) = match app.view {
        View::Machines => machine_detail(app),
        View::Activity => operation_detail(app),
    };
    let panel = block(
        if app.view == View::Machines {
            " Machine details ".into()
        } else {
            title
        },
        app.detail_focus,
    );
    let mut inner = panel.inner(area);
    app.hits.push((inner, Click::Pane(true)));
    frame.render_widget(panel, area);
    if app.view == View::Machines && inner.height > 0 {
        buttons(
            frame,
            app,
            Rect::new(inner.x, inner.y, inner.width, 1),
            &[
                ("Overview", KeyCode::Char('1')),
                ("Definition", KeyCode::Char('2')),
                ("Activity", KeyCode::Char('3')),
            ],
        );
        inner.y += 1;
        inner.height -= 1;
    }
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    // Clamp scrolling after resize or a shorter server reply so the user cannot
    // get stranded beyond the end of a definition or operation detail.
    app.scroll = app.scroll.min(
        paragraph
            .line_count(inner.width)
            .saturating_sub(inner.height as usize)
            .min(u16::MAX as usize) as u16,
    );
    frame.render_widget(paragraph.scroll((app.scroll, 0)), inner);
}

fn machine_detail(app: &App) -> (String, Vec<Line<'static>>) {
    let Some(e) = app.environment() else {
        return (" Details ".into(), vec![muted("Select an environment.")]);
    };
    let title = if app.definition {
        " 1 Overview · [2 Definition] · 3 Activity "
    } else {
        " [1 Overview] · 2 Definition · 3 Activity "
    }
    .into();
    if app.definition {
        let lines = match &app.detail {
            Some(Ok(detail)) => display(&detail.definition_yaml)
                .lines()
                .map(|s| Line::raw(s.to_owned()))
                .collect(),
            Some(Err(error)) => vec![Line::raw(error.clone())],
            None => vec![muted("Loading saved definition…")],
        };
        return (title, lines);
    }
    let mut lines = vec![
        Line::styled(
            e.definition_name.clone(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        value("Instance", &e.id),
        value("Association", e.status.label()),
        value(
            "Target",
            &format!(
                "{} · VM {}",
                e.provider_reference.node, e.provider_reference.vmid
            ),
        ),
        Line::raw(""),
    ];
    if e.status == Association::Registered {
        match &app.power {
            Some(Ok(power)) => {
                lines.push(value("Observed power", &power.observation.status));
                if let Some(qmp) = &power.observation.qmp_status {
                    lines.push(value("QMP", qmp));
                }
                lines.push(muted("Power is not proof of guest or SSH readiness."));
            }
            Some(Err(error)) => {
                lines.push(value("Power", "unknown"));
                lines.push(Line::styled(
                    error.clone(),
                    Style::default().fg(Color::Yellow),
                ));
            }
            None => lines.push(muted("Power: reading…")),
        }
    } else {
        lines.push(muted(
            "No live VM associated with this record; definition retained.",
        ));
    }
    lines.push(Line::raw(""));
    if let Some(g) = &e.last_guest {
        lines.push(value("Last guest", &format!("{}@{}", g.user, g.address)));
        lines.push(value("Readiness observed", &g.observed_at));
        lines.push(muted(
            "Historical connection hint; verify SSH host identity separately.",
        ));
    } else {
        lines.push(muted("No saved guest address."));
    }
    lines.extend([
        Line::raw(""),
        value("Definition revision", &e.definition_revision.to_string()),
        value("SHA-256", &e.definition_hash),
        value("Created", &e.created_at),
        Line::raw(""),
    ]);
    if let Some(Err(error)) = &app.detail {
        lines.push(Line::styled(
            error.clone(),
            Style::default().fg(Color::Yellow),
        ));
    }
    if let Some(op) = app
        .operations
        .iter()
        .find(|op| op.instance_id.as_ref() == Some(&e.id))
    {
        lines.push(Line::from(vec![
            Span::raw(format!(
                "Latest operation: {} · ",
                op.kind.replace('_', " ")
            )),
            status(op.status),
        ]));
    }
    lines.push(muted("1 overview · 2 saved YAML · 3 machine activity"));
    (title, lines)
}

fn operation_detail(app: &App) -> (String, Vec<Line<'static>>) {
    let title = " Recorded operation ".into();
    let Some(op) = app.operation() else {
        return (title, vec![muted("Select an operation.")]);
    };
    let mut lines = vec![
        Line::styled(
            op.kind.replace('_', " "),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        value("Operation", &op.id),
        Line::from(vec![Span::raw("Status: "), status(op.status)]),
        value("Instance", op.instance_id.as_deref().unwrap_or("none")),
        value("Created", &op.created_at),
        value("Updated", &op.updated_at),
    ];
    if let Some(failure) = &op.failure {
        lines.push(value("Failure", &failure.replace('_', " ")));
    }
    if op.provider_task_known {
        lines.push(muted(
            "A provider task reference is retained on the server.",
        ));
    }
    if matches!(op.status, Status::Uncertain | Status::Interrupted) {
        lines.push(Line::styled(
            "Outcome unconfirmed. This TUI never repeats the action.",
            Style::default().fg(Color::Yellow),
        ));
    }
    if op.provisioning() {
        lines.push(Line::raw(""));
        match &app.progress {
            Some(Ok(progress)) => {
                lines.push(Line::from(vec![
                    Span::raw("Provisioning: "),
                    status(progress.status),
                ]));
                for stage in &progress.stages {
                    lines.push(Line::from(vec![
                        Span::raw(format!(
                            "{}. {} · ",
                            stage.position + 1,
                            stage.kind.replace('_', " ")
                        )),
                        status(stage.status),
                    ]));
                }
                lines.push(muted("Recorded steps; no guest command output is fetched."));
            }
            Some(Err(error)) => lines.push(Line::styled(
                error.clone(),
                Style::default().fg(Color::Yellow),
            )),
            None => lines.push(muted("Reading provisioning stages…")),
        }
    }
    (title, lines)
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
fn help(frame: &mut Frame, app: &mut App) {
    let area = centered(frame.area(), 76, 21);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::raw("m / a              Machines / global Activity"),
            Line::raw("↑ ↓ / j k          Move selection; scroll in detail pane"),
            Line::raw("Tab / ← / →        Switch panes"),
            Line::raw("Enter              Open selected detail"),
            Line::raw("1 / 2 / 3          Machine overview / YAML / its Activity"),
            Line::raw("PageUp / PageDown  Scroll detail; Home returns to top"),
            Line::raw("/                  Search the current list"),
            Line::raw("Esc                Cancel search / clear filter"),
            Line::raw("r                  Refresh now (automatic every 5 seconds)"),
            Line::raw("c                  Reconnect when no read is in progress"),
            Line::raw("q / Ctrl-C         Quit; server operations keep running"),
            Line::raw(""),
            muted("Recent lists contain at most 100 records each."),
            muted("Narrow terminals show one pane; Tab switches it."),
            muted("Errors mark cached lists STALE; power reads remain explicit."),
            muted("This version only reads. Use the native app for VM actions."),
            Line::raw(""),
            Line::raw("Click rows/tabs/buttons; wheel scrolls the pointed pane."),
        ])
        .block(block(" Keyboard help ", true))
        .wrap(Wrap { trim: false }),
        area,
    );
    buttons(
        frame,
        app,
        Rect::new(
            area.x + 1,
            area.bottom() - 2,
            area.width.saturating_sub(2),
            1,
        ),
        &[("Close", KeyCode::Esc)],
    );
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
