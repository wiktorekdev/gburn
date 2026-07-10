use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::Utc;
use rayon::prelude::*;
use serde::Deserialize;

use crate::estimator::{
    empty_usage, estimate_from_signals, estimate_from_updates, SessionUsage,
};
use crate::format::{decode_cwd_folder, parse_iso_ms};

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    #[allow(dead_code)]
    pub dir: PathBuf,
    pub cwd: String,
    #[allow(dead_code)]
    pub cwd_folder: String,
    pub title: String,
    pub model_id: String,
    pub models_used: Vec<String>,
    pub created_at: Option<String>,
    #[allow(dead_code)]
    pub updated_at: Option<String>,
    pub last_active_at: Option<String>,
    pub num_messages: u64,
    pub turn_count: u64,
    pub tool_call_count: u64,
    pub context_tokens_used: u64,
    pub context_window_tokens: u64,
    pub session_duration_seconds: u64,
    pub agent_name: Option<String>,
    pub reasoning_effort: Option<String>,
    pub usage: SessionUsage,
    pub is_subagent: bool,
    pub parent_session_id: Option<String>,
    pub child_session_ids: Vec<String>,
    pub subagent_type: Option<String>,
    pub subagent_description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModelTotals {
    pub input: u64,
    pub output: u64,
    pub cost: f64,
    pub sessions: u64,
}

#[derive(Debug, Clone)]
pub struct ScanTotals {
    pub sessions: usize,
    pub with_detailed: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub by_model: HashMap<String, ModelTotals>,
}

#[derive(Debug, Clone)]
pub struct ProjectTotals {
    pub cwd: String,
    pub sessions: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_cost: f64,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub grok_home: PathBuf,
    pub sessions_dir: PathBuf,
    pub sessions: Vec<SessionRecord>,
    pub totals: ScanTotals,
    pub scanned_at: String,
}

#[derive(Debug, Clone)]
struct SubagentLink {
    parent_session_id: String,
    #[allow(dead_code)]
    child_session_id: String,
    subagent_type: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SummaryJson {
    info: Option<SummaryInfo>,
    session_summary: Option<String>,
    generated_title: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    last_active_at: Option<String>,
    num_messages: Option<u64>,
    current_model_id: Option<String>,
    agent_name: Option<String>,
    reasoning_effort: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SummaryInfo {
    id: Option<String>,
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalsJson {
    turn_count: Option<u64>,
    tool_call_count: Option<u64>,
    context_tokens_used: Option<u64>,
    context_window_tokens: Option<u64>,
    session_duration_seconds: Option<u64>,
    models_used: Option<Vec<String>>,
    primary_model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubagentMetaJson {
    subagent_id: Option<String>,
    parent_session_id: Option<String>,
    child_session_id: Option<String>,
    subagent_type: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Cost,
    Date,
    Input,
    Output,
    Turns,
    Title,
}

impl SortKey {
    pub const ALL: [SortKey; 6] = [
        SortKey::Cost,
        SortKey::Date,
        SortKey::Input,
        SortKey::Output,
        SortKey::Turns,
        SortKey::Title,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SortKey::Cost => "cost",
            SortKey::Date => "date",
            SortKey::Input => "input",
            SortKey::Output => "output",
            SortKey::Turns => "turns",
            SortKey::Title => "title",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRange {
    All,
    Day,
    Week,
    Month,
}

impl TimeRange {
    pub fn label(self) -> &'static str {
        match self {
            TimeRange::All => "all",
            TimeRange::Day => "24h",
            TimeRange::Week => "7d",
            TimeRange::Month => "30d",
        }
    }

    pub fn next(self) -> Self {
        match self {
            TimeRange::All => TimeRange::Day,
            TimeRange::Day => TimeRange::Week,
            TimeRange::Week => TimeRange::Month,
            TimeRange::Month => TimeRange::All,
        }
    }

    pub fn cutoff_ms(self) -> Option<i64> {
        let now = Utc::now().timestamp_millis();
        match self {
            TimeRange::All => None,
            TimeRange::Day => Some(now - 24 * 60 * 60 * 1000),
            TimeRange::Week => Some(now - 7 * 24 * 60 * 60 * 1000),
            TimeRange::Month => Some(now - 30 * 24 * 60 * 60 * 1000),
        }
    }
}

pub fn default_grok_home() -> PathBuf {
    if let Ok(home) = std::env::var("GROK_HOME") {
        return PathBuf::from(home);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".grok")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn resolve_cwd(folder_path: &Path, folder_name: &str) -> String {
    let cwd_file = folder_path.join(".cwd");
    if let Ok(raw) = fs::read_to_string(&cwd_file) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    decode_cwd_folder(folder_name)
}

fn resolve_model_from_chat(chat_path: &Path) -> (Option<String>, HashMap<String, u64>) {
    let mut counts: HashMap<String, u64> = HashMap::new();
    let Ok(file) = File::open(chat_path) else {
        return (None, counts);
    };
    let reader = BufReader::new(file);

    for line in reader.lines().map_while(Result::ok) {
        if !line.contains("model_id") {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let type_ok = obj.get("type").and_then(|v| v.as_str()) == Some("assistant");
        if !type_ok {
            continue;
        }
        if let Some(mid) = obj.get("model_id").and_then(|v| v.as_str()) {
            if !mid.is_empty() {
                *counts.entry(mid.to_string()).or_default() += 1;
            }
        }
    }

    let primary = counts
        .iter()
        .max_by_key(|(_, n)| *n)
        .map(|(id, _)| id.clone());
    (primary, counts)
}

fn resolve_model_id(
    from_chat: Option<&str>,
    current_model_id: Option<&str>,
    primary_model_id: Option<&str>,
    models_used: Option<&[String]>,
) -> String {
    if let Some(id) = from_chat {
        return id.to_string();
    }
    if let Some(id) = current_model_id {
        return id.to_string();
    }
    if let Some(id) = primary_model_id {
        return id.to_string();
    }
    if let Some(list) = models_used {
        if let Some(first) = list.first() {
            return first.clone();
        }
    }
    "unknown".into()
}

struct SubagentIndex {
    by_child: HashMap<String, SubagentLink>,
    children_of: HashMap<String, Vec<String>>,
}

fn index_subagents(sessions_dir: &Path) -> SubagentIndex {
    let mut by_child = HashMap::new();
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();

    let Ok(cwd_folders) = fs::read_dir(sessions_dir) else {
        return SubagentIndex {
            by_child,
            children_of,
        };
    };

    for entry in cwd_folders.flatten() {
        let folder_path = entry.path();
        if !folder_path.is_dir() {
            continue;
        }
        let Ok(session_dirs) = fs::read_dir(&folder_path) else {
            continue;
        };
        for session_entry in session_dirs.flatten() {
            let session_path = session_entry.path();
            if !session_path.is_dir() {
                continue;
            }
            let session_id = session_entry.file_name().to_string_lossy().to_string();
            let sub_root = session_path.join("subagents");
            if !sub_root.is_dir() {
                continue;
            }
            let Ok(child_dirs) = fs::read_dir(&sub_root) else {
                continue;
            };
            for child_entry in child_dirs.flatten() {
                let child_path = child_entry.path();
                if !child_path.is_dir() {
                    continue;
                }
                let child_dir = child_entry.file_name().to_string_lossy().to_string();
                let Some(meta) = read_json::<SubagentMetaJson>(&child_path.join("meta.json"))
                else {
                    continue;
                };
                let parent = meta
                    .parent_session_id
                    .unwrap_or_else(|| session_id.clone());
                let child = meta
                    .child_session_id
                    .or(meta.subagent_id)
                    .unwrap_or(child_dir);
                if parent.is_empty() || child.is_empty() {
                    continue;
                }

                let link = SubagentLink {
                    parent_session_id: parent.clone(),
                    child_session_id: child.clone(),
                    subagent_type: meta.subagent_type,
                    description: meta.description,
                };
                by_child.insert(child.clone(), link);
                let list = children_of.entry(parent).or_default();
                if !list.contains(&child) {
                    list.push(child);
                }
            }
        }
    }

    SubagentIndex {
        by_child,
        children_of,
    }
}

struct SessionJob {
    session_dir: PathBuf,
    cwd_folder: String,
    resolved_cwd: String,
}

fn load_session(
    job: &SessionJob,
    subagents: &SubagentIndex,
) -> Option<SessionRecord> {
    let summary = read_json::<SummaryJson>(&job.session_dir.join("summary.json"))?;
    let signals = read_json::<SignalsJson>(&job.session_dir.join("signals.json"));
    let id = summary
        .info
        .as_ref()
        .and_then(|i| i.id.clone())
        .unwrap_or_else(|| {
            job.session_dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });
    let cwd = summary
        .info
        .as_ref()
        .and_then(|i| i.cwd.clone())
        .unwrap_or_else(|| job.resolved_cwd.clone());

    let (chat_primary, chat_counts) =
        resolve_model_from_chat(&job.session_dir.join("chat_history.jsonl"));
    let model_id = resolve_model_id(
        chat_primary.as_deref(),
        summary.current_model_id.as_deref(),
        signals.as_ref().and_then(|s| s.primary_model_id.as_deref()),
        signals.as_ref().and_then(|s| s.models_used.as_deref()),
    );

    let models_used = if !chat_counts.is_empty() {
        let mut entries: Vec<(String, u64)> = chat_counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.into_iter().map(|(m, _)| m).collect()
    } else if let Some(list) = signals.as_ref().and_then(|s| s.models_used.as_ref()) {
        if !list.is_empty() {
            list.clone()
        } else {
            vec![model_id.clone()]
        }
    } else {
        vec![model_id.clone()]
    };

    let title = summary
        .generated_title
        .or(summary.session_summary)
        .unwrap_or_else(|| id.chars().take(8).collect());

    let weights = if chat_counts.is_empty() {
        None
    } else {
        Some(&chat_counts)
    };

    let usage = match estimate_from_updates(
        &job.session_dir.join("updates.jsonl"),
        Some(&model_id),
        weights,
    ) {
        Some(u) => u,
        None => {
            let ctx = signals
                .as_ref()
                .and_then(|s| s.context_tokens_used)
                .unwrap_or(0);
            let turns = signals.as_ref().and_then(|s| s.turn_count).unwrap_or(0);
            if ctx > 0 || turns > 0 {
                estimate_from_signals(ctx, turns.max(1), Some(&model_id), weights)
            } else {
                empty_usage(Some(&model_id))
            }
        }
    };

    let as_child = subagents.by_child.get(&id);
    let child_ids = subagents
        .children_of
        .get(&id)
        .cloned()
        .unwrap_or_default();

    Some(SessionRecord {
        id,
        dir: job.session_dir.clone(),
        cwd,
        cwd_folder: job.cwd_folder.clone(),
        title,
        model_id,
        models_used,
        created_at: summary.created_at,
        updated_at: summary.updated_at.clone(),
        last_active_at: summary.last_active_at.or(summary.updated_at),
        num_messages: summary.num_messages.unwrap_or(0),
        turn_count: signals.as_ref().and_then(|s| s.turn_count).unwrap_or(0),
        tool_call_count: signals.as_ref().and_then(|s| s.tool_call_count).unwrap_or(0),
        context_tokens_used: signals
            .as_ref()
            .and_then(|s| s.context_tokens_used)
            .unwrap_or(0),
        context_window_tokens: signals
            .as_ref()
            .and_then(|s| s.context_window_tokens)
            .unwrap_or(0),
        session_duration_seconds: signals
            .as_ref()
            .and_then(|s| s.session_duration_seconds)
            .unwrap_or(0),
        agent_name: summary.agent_name,
        reasoning_effort: summary.reasoning_effort,
        usage,
        is_subagent: as_child.is_some(),
        parent_session_id: as_child.map(|c| c.parent_session_id.clone()),
        child_session_ids: child_ids,
        subagent_type: as_child.and_then(|c| c.subagent_type.clone()),
        subagent_description: as_child.and_then(|c| c.description.clone()),
    })
}

pub fn scan_sessions(grok_home: Option<PathBuf>, only_cwd: Option<&str>) -> ScanResult {
    let grok_home = grok_home.unwrap_or_else(default_grok_home);
    let sessions_dir = grok_home.join("sessions");
    let scanned_at = Utc::now().to_rfc3339();

    if !sessions_dir.is_dir() {
        return ScanResult {
            grok_home,
            sessions_dir,
            sessions: Vec::new(),
            totals: empty_totals(),
            scanned_at,
        };
    }

    let subagents = index_subagents(&sessions_dir);

    let mut jobs: Vec<SessionJob> = Vec::new();
    if let Ok(cwd_folders) = fs::read_dir(&sessions_dir) {
        for entry in cwd_folders.flatten() {
            let folder_path = entry.path();
            if !folder_path.is_dir() {
                continue;
            }
            let folder_name = entry.file_name().to_string_lossy().to_string();
            let resolved_cwd = resolve_cwd(&folder_path, &folder_name);

            let Ok(children) = fs::read_dir(&folder_path) else {
                continue;
            };
            for child in children.flatten() {
                let session_dir = child.path();
                if !session_dir.is_dir() {
                    continue;
                }
                if !session_dir.join("summary.json").is_file() {
                    continue;
                }
                jobs.push(SessionJob {
                    session_dir,
                    cwd_folder: folder_name.clone(),
                    resolved_cwd: resolved_cwd.clone(),
                });
            }
        }
    }

    let only = only_cwd.map(|s| s.to_lowercase());

    let mut sessions: Vec<SessionRecord> = jobs
        .par_iter()
        .filter_map(|job| {
            let rec = load_session(job, &subagents)?;
            if let Some(ref want) = only {
                let hit = rec.cwd.to_lowercase().contains(want)
                    || job.resolved_cwd.to_lowercase().contains(want)
                    || decode_cwd_folder(&job.cwd_folder).to_lowercase().contains(want);
                if !hit {
                    return None;
                }
            }
            if !rec.usage.cost.priced {
                return None;
            }
            if rec.usage.input_tokens == 0 && rec.usage.output_tokens == 0 {
                return None;
            }
            Some(rec)
        })
        .collect();

    sessions.sort_by(|a, b| {
        parse_iso_ms(b.last_active_at.as_deref())
            .cmp(&parse_iso_ms(a.last_active_at.as_deref()))
    });

    let totals = compute_totals(&sessions);
    ScanResult {
        grok_home,
        sessions_dir,
        sessions,
        totals,
        scanned_at,
    }
}

fn empty_totals() -> ScanTotals {
    ScanTotals {
        sessions: 0,
        with_detailed: 0,
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
        total_cost: 0.0,
        by_model: HashMap::new(),
    }
}

pub fn compute_totals(sessions: &[SessionRecord]) -> ScanTotals {
    let mut totals = empty_totals();
    totals.sessions = sessions.len();

    for s in sessions {
        if s.usage.has_detailed_usage {
            totals.with_detailed += 1;
        }
        totals.input_tokens += s.usage.input_tokens;
        totals.output_tokens += s.usage.output_tokens;
        totals.total_tokens += s.usage.total_tokens;
        totals.total_cost += s.usage.cost.total_cost;

        if !s.usage.by_model.is_empty() {
            for m in &s.usage.by_model {
                let row = totals.by_model.entry(m.model_id.clone()).or_insert(ModelTotals {
                    input: 0,
                    output: 0,
                    cost: 0.0,
                    sessions: 0,
                });
                row.input += m.input_tokens;
                row.output += m.output_tokens;
                row.cost += m.cost.total_cost;
                row.sessions += 1;
            }
        } else {
            let row = totals
                .by_model
                .entry(s.model_id.clone())
                .or_insert(ModelTotals {
                    input: 0,
                    output: 0,
                    cost: 0.0,
                    sessions: 0,
                });
            row.input += s.usage.input_tokens;
            row.output += s.usage.output_tokens;
            row.cost += s.usage.cost.total_cost;
            row.sessions += 1;
        }
    }

    totals
}

pub fn compute_projects(sessions: &[SessionRecord]) -> Vec<ProjectTotals> {
    let mut map: HashMap<String, ProjectTotals> = HashMap::new();
    for s in sessions {
        let row = map.entry(s.cwd.clone()).or_insert(ProjectTotals {
            cwd: s.cwd.clone(),
            sessions: 0,
            input_tokens: 0,
            output_tokens: 0,
            total_cost: 0.0,
        });
        row.sessions += 1;
        row.input_tokens += s.usage.input_tokens;
        row.output_tokens += s.usage.output_tokens;
        row.total_cost += s.usage.cost.total_cost;
    }
    let mut rows: Vec<ProjectTotals> = map.into_values().collect();
    rows.sort_by(|a, b| {
        b.total_cost
            .partial_cmp(&a.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

pub fn sort_sessions(sessions: &mut [SessionRecord], key: SortKey, desc: bool) {
    sessions.sort_by(|a, b| {
        let cmp = match key {
            SortKey::Cost => a
                .usage
                .cost
                .total_cost
                .partial_cmp(&b.usage.cost.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortKey::Date => parse_iso_ms(a.last_active_at.as_deref())
                .cmp(&parse_iso_ms(b.last_active_at.as_deref())),
            SortKey::Input => a.usage.input_tokens.cmp(&b.usage.input_tokens),
            SortKey::Output => a.usage.output_tokens.cmp(&b.usage.output_tokens),
            SortKey::Turns => a.turn_count.cmp(&b.turn_count),
            SortKey::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        };
        if desc {
            cmp.reverse()
        } else {
            cmp
        }
    });
}

pub fn filter_sessions(
    all: &[SessionRecord],
    query: &str,
    time_range: TimeRange,
    hide_subagents: bool,
) -> Vec<SessionRecord> {
    let q = query.trim().to_lowercase();
    let cutoff = time_range.cutoff_ms();

    all.iter()
        .filter(|s| {
            if hide_subagents && s.is_subagent {
                return false;
            }
            if let Some(min_ms) = cutoff {
                if parse_iso_ms(s.last_active_at.as_deref()) < min_ms {
                    return false;
                }
            }
            if q.is_empty() {
                return true;
            }
            s.title.to_lowercase().contains(&q)
                || s.cwd.to_lowercase().contains(&q)
                || s.model_id.to_lowercase().contains(&q)
                || s.id.to_lowercase().contains(&q)
                || s.models_used.iter().any(|m| m.to_lowercase().contains(&q))
        })
        .cloned()
        .collect()
}
