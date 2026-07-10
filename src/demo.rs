use std::path::PathBuf;

use chrono::Utc;

use crate::estimator::{ModelUsage, SessionUsage};
use crate::pricing::calc_cost;
use crate::scanner::{compute_totals, ScanResult, SessionRecord};

fn session(
    id: &str,
    title: &str,
    cwd: &str,
    model_id: &str,
    input: u64,
    output: u64,
    turns: u64,
    tools: u64,
    when: &str,
    is_subagent: bool,
    parent_session_id: Option<&str>,
    child_session_ids: Vec<&str>,
    agent_name: Option<&str>,
) -> SessionRecord {
    let cost = calc_cost(Some(model_id), input, output);
    let streams = (turns as f64 * 2.5).floor().max(1.0) as u64;
    SessionRecord {
        id: id.into(),
        dir: PathBuf::from(format!("/demo/{id}")),
        cwd: cwd.into(),
        cwd_folder: cwd.into(),
        title: title.into(),
        model_id: model_id.into(),
        models_used: vec![model_id.into()],
        created_at: Some(when.into()),
        updated_at: Some(when.into()),
        last_active_at: Some(when.into()),
        num_messages: turns * 8,
        turn_count: turns,
        tool_call_count: tools,
        context_tokens_used: input.min(280_000),
        context_window_tokens: 500_000,
        session_duration_seconds: turns * 180,
        agent_name: Some(agent_name.unwrap_or("grok-build-plan").into()),
        reasoning_effort: Some("high".into()),
        usage: SessionUsage {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
            streams,
            has_detailed_usage: true,
            streams_detail: Vec::new(),
            by_model: vec![ModelUsage {
                model_id: model_id.into(),
                input_tokens: input,
                output_tokens: output,
                streams,
                weight: 1.0,
                cost: cost.clone(),
            }],
            cost,
            estimation_note: "demo".into(),
        },
        is_subagent,
        parent_session_id: parent_session_id.map(|s| s.into()),
        child_session_ids: child_session_ids.into_iter().map(|s| s.into()).collect(),
        subagent_type: if is_subagent {
            Some("general-purpose".into())
        } else {
            None
        },
        subagent_description: if is_subagent {
            Some("you got this king".into())
        } else {
            None
        },
    }
}

pub fn demo_scan() -> ScanResult {
    let parent_id = "demo-gta-parent";
    let sessions = vec![
        session(
            parent_id,
            "create GTA VI, make no mistakes",
            "~/vibe/gta-vi-final-final",
            "grok-4.5",
            84_200_000,
            412_000,
            96,
            890,
            "2026-07-09T22:10:00Z",
            false,
            None,
            vec!["demo-sub-map", "demo-sub-physics"],
            None,
        ),
        session(
            "demo-uber",
            "rebuild Uber but better, make no mistakes",
            "~/vibe/uber-killer",
            "grok-4.5",
            31_400_000,
            188_000,
            34,
            260,
            "2026-07-09T21:05:00Z",
            false,
            None,
            vec![],
            None,
        ),
        session(
            "demo-saas",
            "one-shot a SaaS that prints money by Friday",
            "~/vibe/mrr-printer",
            "grok-4.5",
            18_900_000,
            141_000,
            22,
            175,
            "2026-07-09T19:40:00Z",
            false,
            None,
            vec![],
            None,
        ),
        session(
            "demo-right",
            "you're absolutely right. now fix production",
            "~/work/prod-on-fire",
            "grok-4.5",
            12_600_000,
            97_000,
            15,
            120,
            "2026-07-09T18:20:00Z",
            false,
            None,
            vec![],
            None,
        ),
        session(
            "demo-sub-map",
            "↳ entire Los Santos map in pure CSS",
            "~/vibe/gta-vi-final-final",
            "grok-4.5",
            4_800_000,
            88_000,
            3,
            55,
            "2026-07-09T21:50:00Z",
            true,
            Some(parent_id),
            vec![],
            None,
        ),
        session(
            "demo-sub-physics",
            "↳ realistic car physics, zero bugs, ship tonight",
            "~/vibe/gta-vi-final-final",
            "grok-4.5",
            3_200_000,
            71_000,
            2,
            40,
            "2026-07-09T21:55:00Z",
            true,
            Some(parent_id),
            vec![],
            None,
        ),
        session(
            "demo-accept",
            "accept all 47 edits · trust the vibe",
            "~/vibe/yolo-pr",
            "grok-4.5",
            2_100_000,
            34_000,
            6,
            28,
            "2026-07-09T16:15:00Z",
            false,
            None,
            vec![],
            None,
        ),
        session(
            "demo-opus",
            "build Claude Opus 5, make no mistakes",
            "~/vibe/train-frontier",
            "grok-4.5",
            890_000,
            22_000,
            4,
            15,
            "2026-07-09T14:00:00Z",
            false,
            None,
            vec![],
            None,
        ),
        session(
            "demo-build",
            "don't change anything, just make it work",
            "~/work/legacy-spaghetti",
            "grok-build",
            420_000,
            18_500,
            5,
            31,
            "2026-07-09T12:30:00Z",
            false,
            None,
            vec![],
            Some("grok-build"),
        ),
        session(
            "demo-composer",
            "write the whole monorepo in one message",
            "~/vibe/monorepo-one-shot",
            "grok-composer-2.5-fast",
            180_000,
            41_000,
            2,
            8,
            "2026-07-09T11:05:00Z",
            false,
            None,
            vec![],
            None,
        ),
        session(
            "demo-rate",
            "continue. ignore rate limits. ship.",
            "~/vibe/keep-going",
            "grok-4.5",
            95_000,
            12_000,
            3,
            6,
            "2026-07-09T10:20:00Z",
            false,
            None,
            vec![],
            None,
        ),
        session(
            "demo-senior",
            "act as a 10x senior. no junior code. go.",
            "~/vibe/10x-only",
            "grok-4.5",
            55_000,
            9_800,
            2,
            4,
            "2026-07-09T09:45:00Z",
            false,
            None,
            vec![],
            None,
        ),
    ];

    ScanResult {
        grok_home: PathBuf::from("~/.grok"),
        sessions_dir: PathBuf::from("~/.grok/sessions"),
        totals: compute_totals(&sessions),
        sessions,
        scanned_at: Utc::now().to_rfc3339(),
    }
}

const DEBUG_TITLES: &[&str] = &[
    "ship the whole product by friday",
    "refactor everything, make no mistakes",
    "one-shot the monorepo migration",
    "fix production, you're absolutely right",
    "build GTA VI in pure CSS",
    "train frontier model over lunch",
    "accept all edits · trust the vibe",
    "debug the race condition that only happens on Tuesdays",
    "rewrite auth for the third time this week",
    "optimize until the profiler stops crying",
    "design the landing page that prints money",
    "port legacy SOAP to something human",
    "make the tests green without reading them",
    "add dark mode (how hard could it be)",
    "recover the deleted branch from vibes alone",
    "close all the issues labeled 'quick win'",
    "wire up billing before the demo",
    "explain the architecture to future me",
    "remove the feature flag that became permanent",
    "stop the memory leak at 3am",
];

const DEBUG_CWDS: &[&str] = &[
    "~/vibe/ship-it",
    "~/work/prod-on-fire",
    "~/vibe/monorepo",
    "~/work/legacy",
    "~/vibe/saas-printer",
    "~/work/infra",
    "~/vibe/mobile",
    "~/work/dashboard",
];

/// Fake grok-4.5 sessions for `--debug <usd> <n>`.
/// Total list-price cost ≈ `total_usd`, all model_id = grok-4.5.
pub fn debug_scan(total_usd: f64, n: usize) -> ScanResult {
    let n = n.max(1);
    let total_usd = total_usd.max(0.01);
    let model = "grok-4.5";

    // power-law weights so a few sessions dominate burn
    let weights: Vec<f64> = (0..n)
        .map(|i| 1.0 / ((i as f64) + 1.0).powf(1.05))
        .collect();
    let wsum: f64 = weights.iter().sum();

    let mut shares = Vec::with_capacity(n);
    let mut assigned = 0.0;
    for i in 0..n {
        let s = if i + 1 == n {
            (total_usd - assigned).max(0.01)
        } else {
            let s = total_usd * weights[i] / wsum;
            assigned += s;
            s
        };
        shares.push(s);
    }

    // tiny residual fix so sum of costs matches target after token rounding
    let mut sessions: Vec<SessionRecord> = Vec::with_capacity(n);
    for (i, share) in shares.iter().enumerate() {
        // grok-4.5: $2/M in, $6/M out — bias toward input tokens
        let in_frac = 0.68 + ((i * 7) % 20) as f64 * 0.01;
        let in_cost = share * in_frac;
        let out_cost = share - in_cost;
        let mut input = ((in_cost / 2.0) * 1_000_000.0).round().max(1.0) as u64;
        let mut output = ((out_cost / 6.0) * 1_000_000.0).round() as u64;

        // last session: nudge tokens so total cost is exact-ish
        if i + 1 == n {
            let so_far: f64 = sessions.iter().map(|s| s.usage.cost.total_cost).sum();
            let need = (total_usd - so_far).max(0.01);
            let in_cost = need * 0.75;
            let out_cost = need - in_cost;
            input = ((in_cost / 2.0) * 1_000_000.0).round().max(1.0) as u64;
            output = ((out_cost / 6.0) * 1_000_000.0).round() as u64;
        }

        let title = DEBUG_TITLES[i % DEBUG_TITLES.len()];
        let cwd = DEBUG_CWDS[i % DEBUG_CWDS.len()];
        let hours_ago = (i as i64 + 1) * 3;
        let when = chrono::Utc::now() - chrono::Duration::hours(hours_ago);
        let turns = (8 + i * 3).min(120) as u64;
        let tools = turns * 4 + (i as u64 * 7);

        sessions.push(session(
            &format!("debug-{i:04}"),
            title,
            cwd,
            model,
            input,
            output,
            turns,
            tools,
            &when.to_rfc3339(),
            false,
            None,
            vec![],
            Some("grok-build-plan"),
        ));
    }

    // final scale if still off by more than a cent
    let actual: f64 = sessions.iter().map(|s| s.usage.cost.total_cost).sum();
    if actual > 0.0 && (actual - total_usd).abs() > 0.02 {
        let scale = total_usd / actual;
        for s in &mut sessions {
            let input = ((s.usage.input_tokens as f64) * scale).round().max(1.0) as u64;
            let output = ((s.usage.output_tokens as f64) * scale).round() as u64;
            let cost = calc_cost(Some(model), input, output);
            let streams = s.usage.streams;
            s.usage = SessionUsage {
                input_tokens: input,
                output_tokens: output,
                total_tokens: input + output,
                streams,
                has_detailed_usage: true,
                streams_detail: Vec::new(),
                by_model: vec![ModelUsage {
                    model_id: model.into(),
                    input_tokens: input,
                    output_tokens: output,
                    streams,
                    weight: 1.0,
                    cost: cost.clone(),
                }],
                cost,
                estimation_note: "debug".into(),
            };
            s.context_tokens_used = input.min(280_000);
        }
    }

    // ensure all model ids are grok-4.5 only
    for s in &mut sessions {
        s.model_id = model.into();
        s.models_used = vec![model.into()];
    }

    sessions.sort_by(|a, b| {
        b.usage
            .cost
            .total_cost
            .partial_cmp(&a.usage.cost.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    ScanResult {
        grok_home: PathBuf::from("~/.grok"),
        sessions_dir: PathBuf::from("~/.grok/sessions (debug)"),
        totals: compute_totals(&sessions),
        sessions,
        scanned_at: Utc::now().to_rfc3339(),
    }
}
