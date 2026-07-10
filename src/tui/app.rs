use std::time::Instant;

use crate::scanner::{
    compute_projects, compute_totals, filter_sessions, sort_sessions, ProjectTotals, ScanResult,
    ScanTotals, SessionRecord, SortKey, TimeRange,
};

use super::hit::HitMap;

/// Random intro at process start — plays once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowKind {
    Rocket,
    Casino,
    /// big-burn fire columns
    Inferno,
}

impl ShowKind {
    pub fn pick(total_usd: f64) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let pool: &[ShowKind] = if total_usd >= 5_000.0 {
            &[
                ShowKind::Rocket,
                ShowKind::Casino,
                ShowKind::Inferno,
                ShowKind::Inferno,
                ShowKind::Casino,
            ]
        } else if total_usd >= 500.0 {
            &[ShowKind::Rocket, ShowKind::Casino, ShowKind::Inferno]
        } else {
            &[ShowKind::Rocket, ShowKind::Casino]
        };

        pool[(seed as usize) % pool.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Overall,
    List,
    Detail,
    Pricing,
    Models,
    ModelDetail,
    Projects,
    ProjectDetail,
    Help,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Overall => "overall",
            View::List => "sessions",
            View::Detail => "detail",
            View::Pricing => "pricing",
            View::Models => "models",
            View::ModelDetail => "model",
            View::Projects => "projects",
            View::ProjectDetail => "project",
            View::Help => "help",
        }
    }

    pub fn cycle_order() -> &'static [View] {
        &[
            View::Overall,
            View::List,
            View::Projects,
            View::Models,
            View::Pricing,
            View::Help,
        ]
    }

    pub fn from_digit(d: char) -> Option<View> {
        match d {
            '1' => Some(View::Overall),
            '2' => Some(View::List),
            '3' => Some(View::Projects),
            '4' => Some(View::Models),
            '5' => Some(View::Pricing),
            '6' => Some(View::Help),
            _ => None,
        }
    }

    pub fn is_nav_list(self) -> bool {
        matches!(self, View::List | View::Projects | View::Models)
    }
}

pub struct App {
    pub view: View,
    pub all_sessions: Vec<SessionRecord>,
    pub sessions: Vec<SessionRecord>,
    pub scan: ScanResult,
    pub sort_key: SortKey,
    pub sort_desc: bool,
    pub query: String,
    pub searching: bool,
    pub detail_scroll: u16,
    pub detail_max_scroll: u16,
    pub time_range: TimeRange,
    pub hide_subagents: bool,
    pub should_quit: bool,
    pub selected: usize,
    pub list_scroll: usize,
    pub list_height: usize,
    pub last_click_at: Instant,
    pub last_click_index: Option<usize>,
    pub hits: HitMap,
    pub dragging: bool,
    pub anim_frame: u64,
    pub last_tick: Instant,
    pub reveal: f64,
    pub show_done: bool,
    pub show_kind: ShowKind,
    pub top_burner_ids: Vec<String>,
    /// index into projects() / by_model rows
    pub selected_project: usize,
    pub selected_model: usize,
    pub project_scroll: usize,
    pub model_scroll: usize,
}

impl App {
    pub fn new(scan: ScanResult) -> Self {
        let mut app = Self {
            view: View::Overall,
            all_sessions: scan.sessions.clone(),
            sessions: Vec::new(),
            scan,
            sort_key: SortKey::Cost,
            sort_desc: true,
            query: String::new(),
            searching: false,
            detail_scroll: 0,
            detail_max_scroll: 0,
            time_range: TimeRange::All,
            hide_subagents: false,
            should_quit: false,
            selected: 0,
            list_scroll: 0,
            list_height: 10,
            last_click_at: Instant::now(),
            last_click_index: None,
            hits: HitMap::default(),
            dragging: false,
            anim_frame: 0,
            last_tick: Instant::now(),
            reveal: 0.0,
            show_done: false,
            show_kind: ShowKind::Rocket,
            top_burner_ids: Vec::new(),
            selected_project: 0,
            selected_model: 0,
            project_scroll: 0,
            model_scroll: 0,
        };
        app.apply_filter();
        let total = app.filtered_totals().total_cost;
        app.show_kind = ShowKind::pick(total);
        app
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_tick).as_millis() >= 40 {
            self.anim_frame = self.anim_frame.wrapping_add(1);
            self.last_tick = now;
            if !self.show_done && self.reveal < 1.0 {
                self.reveal = (self.reveal + 0.007).min(1.0);
                if self.reveal >= 1.0 {
                    self.show_done = true;
                    self.reveal = 1.0;
                }
            }
        }
    }

    pub fn reveal_eased(&self) -> f64 {
        let t = self.reveal.clamp(0.0, 1.0);
        if self.show_done || t >= 1.0 {
            return 1.0;
        }
        match self.show_kind {
            ShowKind::Rocket => {
                if t < 0.18 {
                    t * 0.04
                } else {
                    let u = ((t - 0.18) / 0.82).clamp(0.0, 1.0);
                    let s = u * u * (3.0 - 2.0 * u);
                    0.04 + 0.96 * s
                }
            }
            ShowKind::Casino => {
                if t < 0.28 {
                    t * 0.06
                } else {
                    let u = ((t - 0.28) / 0.72).clamp(0.0, 1.0);
                    0.06 + 0.94 * (1.0 - (1.0 - u).powi(2))
                }
            }
            ShowKind::Inferno => {
                if t < 0.22 {
                    t * 0.05
                } else {
                    let u = ((t - 0.22) / 0.78).clamp(0.0, 1.0);
                    0.05 + 0.95 * (u * u)
                }
            }
        }
    }

    pub fn open_session_by_id(&mut self, id: &str) {
        if let Some(idx) = self.sessions.iter().position(|s| s.id == id) {
            self.select(idx);
            self.view = View::Detail;
            self.detail_scroll = 0;
            return;
        }
        if self.all_sessions.iter().any(|s| s.id == id) {
            self.sessions = self.all_sessions.clone();
            sort_sessions(&mut self.sessions, self.sort_key, self.sort_desc);
            if let Some(idx) = self.sessions.iter().position(|s| s.id == id) {
                self.select(idx);
                self.view = View::Detail;
                self.detail_scroll = 0;
            }
        }
    }

    pub fn reload(&mut self, scan: ScanResult) {
        self.scan = scan;
        self.all_sessions = self.scan.sessions.clone();
        self.apply_filter();
    }

    pub fn apply_filter(&mut self) {
        let mut list = filter_sessions(
            &self.all_sessions,
            &self.query,
            self.time_range,
            self.hide_subagents,
        );
        sort_sessions(&mut list, self.sort_key, self.sort_desc);
        self.sessions = list;

        if self.sessions.is_empty() {
            self.selected = 0;
            self.list_scroll = 0;
        } else {
            self.selected = self.selected.min(self.sessions.len() - 1);
            self.ensure_visible();
        }

        self.top_burner_ids = self
            .top_burners(8)
            .into_iter()
            .map(|s| s.id.clone())
            .collect();

        let projects = self.projects();
        if projects.is_empty() {
            self.selected_project = 0;
            self.project_scroll = 0;
        } else {
            self.selected_project = self.selected_project.min(projects.len() - 1);
        }

        let models = self.model_rows();
        if models.is_empty() {
            self.selected_model = 0;
            self.model_scroll = 0;
        } else {
            self.selected_model = self.selected_model.min(models.len() - 1);
        }
    }

    pub fn selected_session(&self) -> Option<&SessionRecord> {
        self.sessions.get(self.selected)
    }

    pub fn select(&mut self, idx: usize) {
        if self.sessions.is_empty() {
            return;
        }
        self.selected = idx.min(self.sessions.len() - 1);
        self.ensure_visible();
    }

    pub fn open_detail(&mut self) {
        match self.view {
            View::List if !self.sessions.is_empty() => {
                self.view = View::Detail;
                self.detail_scroll = 0;
            }
            View::Projects if !self.projects().is_empty() => {
                self.view = View::ProjectDetail;
                self.detail_scroll = 0;
            }
            View::Models if !self.model_rows().is_empty() => {
                self.view = View::ModelDetail;
                self.detail_scroll = 0;
            }
            View::Overall if !self.sessions.is_empty() => {
                self.view = View::Detail;
                self.detail_scroll = 0;
            }
            _ => {}
        }
    }

    pub fn go_back(&mut self) {
        if self.searching {
            self.searching = false;
            self.query.clear();
            self.apply_filter();
            return;
        }
        match self.view {
            View::Detail => {
                self.view = View::List;
                self.detail_scroll = 0;
            }
            View::ProjectDetail => {
                self.view = View::Projects;
                self.detail_scroll = 0;
            }
            View::ModelDetail => {
                self.view = View::Models;
                self.detail_scroll = 0;
            }
            View::List | View::Overall => {}
            _ => {
                self.view = View::Overall;
                self.detail_scroll = 0;
            }
        }
    }

    pub fn set_view(&mut self, view: View) {
        self.view = view;
        self.detail_scroll = 0;
        self.searching = false;
    }

    pub fn move_up(&mut self, n: usize) {
        match self.view {
            View::List => self.select(self.selected.saturating_sub(n)),
            View::Projects => {
                self.selected_project = self.selected_project.saturating_sub(n);
                self.ensure_project_visible();
            }
            View::Models => {
                self.selected_model = self.selected_model.saturating_sub(n);
                self.ensure_model_visible();
            }
            View::Overall => {}
            _ => self.detail_scroll = self.detail_scroll.saturating_sub(n as u16),
        }
    }

    pub fn move_down(&mut self, n: usize) {
        match self.view {
            View::List => {
                if self.sessions.is_empty() {
                    return;
                }
                self.select(self.selected.saturating_add(n));
            }
            View::Projects => {
                let len = self.projects().len();
                if len == 0 {
                    return;
                }
                self.selected_project = (self.selected_project + n).min(len - 1);
                self.ensure_project_visible();
            }
            View::Models => {
                let len = self.model_rows().len();
                if len == 0 {
                    return;
                }
                self.selected_model = (self.selected_model + n).min(len - 1);
                self.ensure_model_visible();
            }
            View::Overall => {}
            _ => {
                self.detail_scroll =
                    (self.detail_scroll.saturating_add(n as u16)).min(self.detail_max_scroll);
            }
        }
    }

    pub fn scroll_list(&mut self, delta: i32) {
        if self.view == View::Projects {
            if delta < 0 {
                self.move_up((-delta) as usize);
            } else {
                self.move_down(delta as usize);
            }
            return;
        }
        if self.view == View::Models {
            if delta < 0 {
                self.move_up((-delta) as usize);
            } else {
                self.move_down(delta as usize);
            }
            return;
        }
        if self.sessions.is_empty() {
            return;
        }
        let max_scroll = self.sessions.len().saturating_sub(self.list_height.max(1));
        if delta < 0 {
            self.list_scroll = self.list_scroll.saturating_sub((-delta) as usize);
        } else {
            self.list_scroll = (self.list_scroll + delta as usize).min(max_scroll);
        }
        if self.selected < self.list_scroll {
            self.selected = self.list_scroll;
        } else if self.selected >= self.list_scroll + self.list_height {
            self.selected = self.list_scroll + self.list_height - 1;
        }
    }

    pub fn jump_top(&mut self) {
        match self.view {
            View::List => self.select(0),
            View::Projects => {
                self.selected_project = 0;
                self.project_scroll = 0;
            }
            View::Models => {
                self.selected_model = 0;
                self.model_scroll = 0;
            }
            _ => self.detail_scroll = 0,
        }
    }

    pub fn jump_bottom(&mut self) {
        match self.view {
            View::List if !self.sessions.is_empty() => self.select(self.sessions.len() - 1),
            View::Projects => {
                let len = self.projects().len();
                if len > 0 {
                    self.selected_project = len - 1;
                    self.ensure_project_visible();
                }
            }
            View::Models => {
                let len = self.model_rows().len();
                if len > 0 {
                    self.selected_model = len - 1;
                    self.ensure_model_visible();
                }
            }
            _ => self.detail_scroll = self.detail_max_scroll,
        }
    }

    pub fn cycle_sort(&mut self) {
        // cost↓ → cost↑ → date↓ → date↑ → …
        if self.sort_desc {
            self.sort_desc = false;
        } else {
            self.sort_key = self.sort_key.next();
            self.sort_desc = true;
        }
        self.apply_filter();
    }

    pub fn sort_chip_label(&self) -> String {
        let dir = if self.sort_desc { "↓" } else { "↑" };
        format!(" {}{} ", self.sort_key.label(), dir)
    }

    pub fn cycle_view(&mut self) {
        let order = View::cycle_order();
        let current = match self.view {
            View::Detail => View::List,
            View::ProjectDetail => View::Projects,
            View::ModelDetail => View::Models,
            other => other,
        };
        let idx = order.iter().position(|v| *v == current).unwrap_or(0);
        self.set_view(order[(idx + 1) % order.len()]);
    }

    pub fn filtered_totals(&self) -> ScanTotals {
        compute_totals(&self.sessions)
    }

    pub fn projects(&self) -> Vec<ProjectTotals> {
        compute_projects(&self.sessions)
    }

    pub fn model_rows(&self) -> Vec<(String, crate::scanner::ModelTotals)> {
        let totals = self.filtered_totals();
        let mut rows: Vec<_> = totals.by_model.into_iter().collect();
        rows.sort_by(|a, b| {
            b.1.cost
                .partial_cmp(&a.1.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        rows
    }

    pub fn current_project(&self) -> Option<ProjectTotals> {
        self.projects().into_iter().nth(self.selected_project)
    }

    pub fn current_model(&self) -> Option<(String, crate::scanner::ModelTotals)> {
        self.model_rows().into_iter().nth(self.selected_model)
    }

    pub fn sessions_for_project(&self, cwd: &str) -> Vec<&SessionRecord> {
        self.sessions.iter().filter(|s| s.cwd == cwd).collect()
    }

    pub fn sessions_for_model(&self, model_id: &str) -> Vec<&SessionRecord> {
        self.sessions
            .iter()
            .filter(|s| s.model_id == model_id || s.models_used.iter().any(|m| m == model_id))
            .collect()
    }

    pub fn burn_per_day(&self) -> f64 {
        let mut min_ms = i64::MAX;
        let mut max_ms = i64::MIN;
        for s in &self.sessions {
            let ms = crate::format::parse_iso_ms(s.last_active_at.as_deref());
            if ms == 0 {
                continue;
            }
            min_ms = min_ms.min(ms);
            max_ms = max_ms.max(ms);
        }
        if min_ms == i64::MAX || max_ms <= min_ms {
            return self.filtered_totals().total_cost;
        }
        let days = ((max_ms - min_ms) as f64 / (24.0 * 60.0 * 60.0 * 1000.0)).max(1.0 / 24.0);
        self.filtered_totals().total_cost / days
    }

    pub fn top_burners(&self, n: usize) -> Vec<&SessionRecord> {
        let mut v: Vec<&SessionRecord> = self.sessions.iter().collect();
        v.sort_by(|a, b| {
            b.usage
                .cost
                .total_cost
                .partial_cmp(&a.usage.cost.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v.into_iter().take(n).collect()
    }

    pub fn ensure_visible(&mut self) {
        let h = self.list_height.max(1);
        if self.selected < self.list_scroll {
            self.list_scroll = self.selected;
        } else if self.selected >= self.list_scroll + h {
            self.list_scroll = self.selected + 1 - h;
        }
    }

    pub fn ensure_project_visible(&mut self) {
        let h = self.list_height.max(1);
        if self.selected_project < self.project_scroll {
            self.project_scroll = self.selected_project;
        } else if self.selected_project >= self.project_scroll + h {
            self.project_scroll = self.selected_project + 1 - h;
        }
    }

    pub fn ensure_model_visible(&mut self) {
        let h = self.list_height.max(1);
        if self.selected_model < self.model_scroll {
            self.model_scroll = self.selected_model;
        } else if self.selected_model >= self.model_scroll + h {
            self.model_scroll = self.selected_model + 1 - h;
        }
    }
}
