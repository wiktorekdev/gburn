use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde_json::Value;

use crate::pricing::{calc_cost, CostBreakdown};

const AGENT_TYPES: &[&str] = &[
    "AgentThoughtChunk",
    "AgentMessageChunk",
    "ToolCall",
    "Plan",
    "agent_thought_chunk",
    "agent_message_chunk",
    "tool_call",
    "plan",
];

#[derive(Debug, Clone)]
pub struct StreamUsage {
    #[allow(dead_code)]
    pub stream_start_ms: Option<i64>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[allow(dead_code)]
    pub update_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModelUsage {
    pub model_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[allow(dead_code)]
    pub streams: u64,
    pub weight: f64,
    pub cost: CostBreakdown,
}

#[derive(Debug, Clone)]
pub struct SessionUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub streams: u64,
    pub has_detailed_usage: bool,
    pub streams_detail: Vec<StreamUsage>,
    pub by_model: Vec<ModelUsage>,
    pub cost: CostBreakdown,
    pub estimation_note: String,
}

struct StreamEvent {
    tt: u64,
    type_name: String,
    stream_start_ms: Option<i64>,
}

fn extract_meta(obj: &Value) -> Option<(u64, String, Option<i64>)> {
    let params = obj.get("params")?;
    let meta = params.get("_meta")?;
    let total_tokens = meta.get("totalTokens")?.as_u64()?;

    let update = params.get("update");
    let session_update = update
        .and_then(|u| u.get("sessionUpdate"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let update_type = meta
        .get("updateType")
        .and_then(|v| v.as_str())
        .unwrap_or(session_update)
        .to_string();

    let stream_start_ms = meta.get("streamStartMs").and_then(|v| v.as_i64());

    Some((total_tokens, update_type, stream_start_ms))
}

pub fn estimate_from_updates(
    updates_path: &Path,
    primary_model_id: Option<&str>,
    model_weights: Option<&HashMap<String, u64>>,
) -> Option<SessionUsage> {
    let file = File::open(updates_path).ok()?;
    let reader = BufReader::new(file);

    let mut by_stream: HashMap<String, Vec<StreamEvent>> = HashMap::new();
    let mut fallback_idx: u64 = 0;
    let mut saw_tokens = false;

    for line in reader.lines().map_while(Result::ok) {
        if !line.contains("totalTokens") {
            continue;
        }
        let obj: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some((tt, type_name, stream_start_ms)) = extract_meta(&obj) else {
            continue;
        };
        saw_tokens = true;

        let key = match stream_start_ms {
            Some(ms) => ms.to_string(),
            None => {
                let k = format!("anon-{fallback_idx}");
                fallback_idx += 1;
                k
            }
        };

        by_stream.entry(key).or_default().push(StreamEvent {
            tt,
            type_name,
            stream_start_ms,
        });
    }

    if !saw_tokens || by_stream.is_empty() {
        return None;
    }

    let mut streams_detail = Vec::new();
    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;

    for evs in by_stream.values() {
        if evs.is_empty() {
            continue;
        }
        let first = evs[0].tt;
        let mut types: Vec<String> = evs
            .iter()
            .map(|e| e.type_name.clone())
            .filter(|t| !t.is_empty())
            .collect();
        types.sort();
        types.dedup();

        let agent_tts: Vec<u64> = evs
            .iter()
            .filter(|e| AGENT_TYPES.contains(&e.type_name.as_str()))
            .map(|e| e.tt)
            .collect();

        let out = if !agent_tts.is_empty() {
            agent_tts.iter().copied().max().unwrap_or(first).saturating_sub(first)
        } else {
            let raw = evs.iter().map(|e| e.tt).max().unwrap_or(first).saturating_sub(first);
            if raw > 0 {
                (raw as f64 * 0.15).floor() as u64
            } else {
                0
            }
        };

        input_tokens += first;
        output_tokens += out;
        streams_detail.push(StreamUsage {
            stream_start_ms: evs[0].stream_start_ms,
            input_tokens: first,
            output_tokens: out,
            update_types: types,
        });
    }

    Some(finalize_usage(
        input_tokens,
        output_tokens,
        streams_detail.len() as u64,
        streams_detail,
        true,
        primary_model_id,
        model_weights,
        "Estimated from updates.jsonl totalTokens per stream (input @ stream start, output ≈ agent growth). Multi-model sessions split by chat_history message share. Not official billing.",
    ))
}

pub fn estimate_from_signals(
    context_tokens_used: u64,
    turn_count: u64,
    primary_model_id: Option<&str>,
    model_weights: Option<&HashMap<String, u64>>,
) -> SessionUsage {
    let avg_input = if context_tokens_used > 0 {
        (context_tokens_used as f64 * 0.55).floor() as u64
    } else {
        0
    };
    let turns = turn_count.max(1);
    let input_tokens = avg_input.saturating_mul(turns);

    finalize_usage(
        input_tokens,
        0,
        turns,
        Vec::new(),
        false,
        primary_model_id,
        model_weights,
        "Rough fallback from signals.json (peak context × turns). Output unknown - lower bound. Multi-model split by chat share when available.",
    )
}

pub fn empty_usage(primary_model_id: Option<&str>) -> SessionUsage {
    finalize_usage(
        0,
        0,
        0,
        Vec::new(),
        false,
        primary_model_id,
        None,
        "No usage data found for this session.",
    )
}

fn finalize_usage(
    input_tokens: u64,
    output_tokens: u64,
    streams: u64,
    streams_detail: Vec<StreamUsage>,
    has_detailed_usage: bool,
    primary_model_id: Option<&str>,
    model_weights: Option<&HashMap<String, u64>>,
    estimation_note: &str,
) -> SessionUsage {
    let by_model = split_by_model(
        input_tokens,
        output_tokens,
        streams,
        primary_model_id,
        model_weights,
    );

    let total_cost: f64 = by_model.iter().map(|m| m.cost.total_cost).sum();
    let total_in_cost: f64 = by_model.iter().map(|m| m.cost.input_cost).sum();
    let total_out_cost: f64 = by_model.iter().map(|m| m.cost.output_cost).sum();
    let any_priced = by_model.iter().any(|m| m.cost.priced);
    let primary = primary_model_id
        .map(|s| s.to_string())
        .or_else(|| by_model.first().map(|m| m.model_id.clone()))
        .unwrap_or_else(|| "unknown".into());
    let primary_price = calc_cost(Some(&primary), input_tokens, output_tokens);

    SessionUsage {
        input_tokens,
        output_tokens,
        total_tokens: input_tokens + output_tokens,
        streams,
        has_detailed_usage,
        streams_detail,
        by_model,
        cost: CostBreakdown {
            input_cost: total_in_cost,
            output_cost: total_out_cost,
            total_cost,
            priced: any_priced,
            price: primary_price.price,
        },
        estimation_note: estimation_note.to_string(),
    }
}

pub fn split_by_model(
    input_tokens: u64,
    output_tokens: u64,
    streams: u64,
    primary_model_id: Option<&str>,
    model_weights: Option<&HashMap<String, u64>>,
) -> Vec<ModelUsage> {
    let mut weights: HashMap<String, u64> = HashMap::new();

    if let Some(mw) = model_weights {
        for (id, w) in mw {
            if *w > 0 {
                weights.insert(id.clone(), *w);
            }
        }
    }

    if weights.is_empty() {
        let id = primary_model_id.unwrap_or("unknown").to_string();
        weights.insert(id, 1);
    }

    let total_w: u64 = weights.values().sum();
    if total_w == 0 {
        let id = primary_model_id.unwrap_or("unknown").to_string();
        let cost = calc_cost(Some(&id), input_tokens, output_tokens);
        return vec![ModelUsage {
            model_id: id,
            input_tokens,
            output_tokens,
            streams,
            weight: 1.0,
            cost,
        }];
    }

    let entries: Vec<(String, u64)> = weights.into_iter().collect();
    let mut result = Vec::with_capacity(entries.len());
    let mut assigned_in = 0u64;
    let mut assigned_out = 0u64;
    let mut assigned_streams = 0u64;

    for (i, (model_id, w)) in entries.iter().enumerate() {
        let share = *w as f64 / total_w as f64;
        let is_last = i == entries.len() - 1;

        let in_tok = if is_last {
            input_tokens.saturating_sub(assigned_in)
        } else {
            (input_tokens as f64 * share).floor() as u64
        };
        let out_tok = if is_last {
            output_tokens.saturating_sub(assigned_out)
        } else {
            (output_tokens as f64 * share).floor() as u64
        };
        let stream_n = if is_last {
            streams.saturating_sub(assigned_streams)
        } else {
            (streams as f64 * share).floor() as u64
        };

        assigned_in += in_tok;
        assigned_out += out_tok;
        assigned_streams += stream_n;

        result.push(ModelUsage {
            model_id: model_id.clone(),
            input_tokens: in_tok,
            output_tokens: out_tok,
            streams: stream_n,
            weight: share,
            cost: calc_cost(Some(model_id), in_tok, out_tok),
        });
    }

    result.sort_by(|a, b| {
        b.cost
            .total_cost
            .partial_cmp(&a.cost.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}
