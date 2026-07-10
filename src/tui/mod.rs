mod app;
mod hit;
mod ui;

use std::io::stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::demo::{debug_scan, demo_scan};
use crate::scanner::{scan_sessions, ScanResult};

use app::{App, View};
use hit::{ChipAction, OverallAction};

pub enum DataSource {
    Live {
        home: Option<PathBuf>,
        cwd: Option<String>,
    },
    Demo,
    Debug {
        usd: f64,
        n: usize,
    },
}

impl DataSource {
    fn rescan(&self) -> ScanResult {
        match self {
            DataSource::Live { home, cwd } => scan_sessions(home.clone(), cwd.as_deref()),
            DataSource::Demo => demo_scan(),
            DataSource::Debug { usd, n } => debug_scan(*usd, *n),
        }
    }
}

pub fn run_tui(initial: ScanResult, source: DataSource) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(initial);
    let tick_rate = Duration::from_millis(33);

    let result = loop {
        app.tick();
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Key(key)
                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat =>
                {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        break Ok(());
                    }
                    handle_key(&mut app, key.code, &source)?;
                    if app.should_quit {
                        break Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(&mut app, mouse);
                    if app.should_quit {
                        break Ok(());
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn handle_key(app: &mut App, code: KeyCode, source: &DataSource) -> anyhow::Result<()> {
    if app.searching {
        match code {
            KeyCode::Esc => app.go_back(),
            KeyCode::Enter => {
                app.searching = false;
                app.apply_filter();
            }
            KeyCode::Backspace => {
                app.query.pop();
                app.apply_filter();
            }
            KeyCode::Char(c) => {
                app.query.push(c);
                app.apply_filter();
            }
            _ => {}
        }
        return Ok(());
    }

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc | KeyCode::Backspace => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') => app.move_up(1),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(1),
        KeyCode::PageUp => app.move_up(10),
        KeyCode::PageDown => app.move_down(10),
        KeyCode::Home | KeyCode::Char('g') => app.jump_top(),
        KeyCode::End | KeyCode::Char('G') => app.jump_bottom(),
        KeyCode::Enter => {
            if app.view == View::Overall {
                app.set_view(View::List);
            } else {
                app.open_detail();
            }
        }
        KeyCode::Char('/') | KeyCode::Char('f') => {
            app.set_view(View::List);
            app.searching = true;
        }
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Char('o') => app.set_view(View::Overall),
        KeyCode::Char('p') => {
            if app.view == View::Pricing {
                app.set_view(View::Overall);
            } else {
                app.set_view(View::Pricing);
            }
        }
        KeyCode::Char('m') => {
            if app.view == View::Models {
                app.set_view(View::Overall);
            } else {
                app.set_view(View::Models);
            }
        }
        KeyCode::Char('P') => {
            if app.view == View::Projects {
                app.set_view(View::Overall);
            } else {
                app.set_view(View::Projects);
            }
        }
        KeyCode::Char('t') => {
            app.time_range = app.time_range.next();
            app.apply_filter();
        }
        KeyCode::Char('a') => {
            app.hide_subagents = !app.hide_subagents;
            app.apply_filter();
        }
        KeyCode::Char('?') => {
            if app.view == View::Help {
                app.set_view(View::Overall);
            } else {
                app.set_view(View::Help);
            }
        }
        KeyCode::Char(d @ '1'..='6') => {
            if let Some(v) = View::from_digit(d) {
                app.set_view(v);
            }
        }
        KeyCode::Tab => app.cycle_view(),
        KeyCode::BackTab => {
            let order = View::cycle_order();
            let current = match app.view {
                View::Detail => View::List,
                View::ProjectDetail => View::Projects,
                View::ModelDetail => View::Models,
                other => other,
            };
            let idx = order.iter().position(|v| *v == current).unwrap_or(0);
            let prev = if idx == 0 { order.len() - 1 } else { idx - 1 };
            app.set_view(order[prev]);
        }
        KeyCode::Char('r') => {
            app.reload(source.rescan());
        }
        _ => {}
    }
    Ok(())
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    let col = mouse.column;
    let row = mouse.row;

    match mouse.kind {
        // ── wheel ──────────────────────────────────────────────
        MouseEventKind::ScrollUp => {
            if app.view == View::List {
                // prefer viewport scroll when pointer is over list
                if app.hits.list.is_some_and(|r| r.contains(col, row)) {
                    app.scroll_list(-3);
                } else {
                    app.move_up(3);
                }
            } else if app
                .hits
                .page_body
                .is_some_and(|r| r.contains(col, row))
                || app.hits.page_body.is_none()
            {
                app.move_up(3);
            }
        }
        MouseEventKind::ScrollDown => {
            if app.view == View::List {
                if app.hits.list.is_some_and(|r| r.contains(col, row)) {
                    app.scroll_list(3);
                } else {
                    app.move_down(3);
                }
            } else {
                app.move_down(3);
            }
        }

        // ── left down ──────────────────────────────────────────
        MouseEventKind::Down(MouseButton::Left) => {
            app.dragging = true;
            // leave search on any click outside is ok via esc; click still works
            if handle_click(app, col, row, true) {
                // double-click handled inside
            }
        }

        // ── left drag — scrub list rows ────────────────────────
        MouseEventKind::Drag(MouseButton::Left) => {
            if !app.dragging {
                return;
            }
            if app.view == View::List {
                if let Some(idx) = app.hits.list_index_at(col, row, app.list_scroll) {
                    if idx < app.sessions.len() {
                        app.select(idx);
                    }
                }
            }
        }

        // ── left up ────────────────────────────────────────────
        MouseEventKind::Up(MouseButton::Left) => {
            app.dragging = false;
        }

        // ── right click = back ─────────────────────────────────
        MouseEventKind::Down(MouseButton::Right) => {
            app.go_back();
        }

        // ── middle click = open detail ─────────────────────────
        MouseEventKind::Down(MouseButton::Middle) => {
            if app.view == View::List {
                if let Some(idx) = app.hits.list_index_at(col, row, app.list_scroll) {
                    if idx < app.sessions.len() {
                        app.select(idx);
                        app.open_detail();
                    }
                } else {
                    app.open_detail();
                }
            }
        }

        _ => {}
    }
}

/// Returns true if something was activated.
fn handle_click(app: &mut App, col: u16, row: u16, check_double: bool) -> bool {
    // 1) tabs — work from any view
    if let Some(view) = app.hits.tab_at(col, row) {
        app.set_view(view);
        return true;
    }

    // 2) chips (sort / time / subs / back)
    if let Some(action) = app.hits.chip_at(col, row) {
        match action {
            ChipAction::Sort => app.cycle_sort(),
            ChipAction::TimeRange => {
                app.time_range = app.time_range.next();
                app.apply_filter();
            }
            ChipAction::Search => {
                app.view = View::List;
                app.searching = true;
            }
        }
        return true;
    }

    // 3) overall interactive zones
    if app.view == View::Overall {
        if let Some(action) = app.hits.overall_at(col, row) {
            match action {
                OverallAction::Range => {
                    app.time_range = app.time_range.next();
                    app.apply_filter();
                }
                OverallAction::Sessions => app.set_view(View::List),
                OverallAction::Models => app.set_view(View::Models),
                OverallAction::Burner(i) => {
                    if let Some(id) = app.top_burner_ids.get(i).cloned() {
                        app.open_session_by_id(&id);
                    }
                }
            }
            return true;
        }
    }

    // 4) footer meta → open detail
    if app
        .hits
        .footer_meta
        .is_some_and(|r| r.contains(col, row))
        && app.view.is_nav_list()
    {
        app.open_detail();
        return true;
    }

    // 5) list rows (sessions / projects / models)
    if app.view.is_nav_list() {
        let scroll = match app.view {
            View::List => app.list_scroll,
            View::Projects => app.project_scroll,
            View::Models => app.model_scroll,
            _ => 0,
        };
        if let Some(idx) = app.hits.list_index_at(col, row, scroll) {
            let len = match app.view {
                View::List => app.sessions.len(),
                View::Projects => app.projects().len(),
                View::Models => app.model_rows().len(),
                _ => 0,
            };
            if idx < len {
                let now = Instant::now();
                let cur = match app.view {
                    View::List => app.selected,
                    View::Projects => app.selected_project,
                    View::Models => app.selected_model,
                    _ => 0,
                };
                let is_double = check_double
                    && app.last_click_index == Some(idx)
                    && now.duration_since(app.last_click_at) < Duration::from_millis(450);
                let reopen = check_double
                    && cur == idx
                    && app.last_click_index == Some(idx)
                    && now.duration_since(app.last_click_at) < Duration::from_millis(450);

                match app.view {
                    View::List => app.select(idx),
                    View::Projects => {
                        app.selected_project = idx;
                        app.ensure_project_visible();
                    }
                    View::Models => {
                        app.selected_model = idx;
                        app.ensure_model_visible();
                    }
                    _ => {}
                }
                app.last_click_at = now;
                app.last_click_index = Some(idx);

                if is_double || reopen {
                    app.open_detail();
                }
                return true;
            }
        }
    }

    false
}
