use crate::format::{format_tokens, format_usd, model_short, truncate};
use crate::pricing::PRICING_SOURCE;
use crate::scanner::{sort_sessions, ScanResult, SortKey, SessionRecord};

fn cost_label(s: &SessionRecord) -> String {
    let n = s.usage.cost.total_cost;
    if s.usage.input_tokens == 0 && s.usage.output_tokens == 0 {
        return "-".into();
    }
    if !s.usage.has_detailed_usage {
        return format!("~{}", format_usd(n));
    }
    format_usd(n)
}

pub fn print_summary(scan: &ScanResult) {
    let totals = &scan.totals;
    println!("gburn  grok build · api cost");
    println!("{}", scan.sessions_dir.display());
    println!();
    println!(
        "sessions {}  cost {}",
        totals.sessions,
        format_usd(totals.total_cost)
    );
    println!(
        "tokens  in {}  out {}  sum {}",
        format_tokens(totals.input_tokens),
        format_tokens(totals.output_tokens),
        format_tokens(totals.total_tokens)
    );
    println!();

    let mut rows = scan.sessions.clone();
    sort_sessions(&mut rows, SortKey::Cost, true);
    let show = rows.iter().take(25);

    println!(
        "{:>10}{:>10}{:>10}  {:<12}{}",
        "COST", "INPUT", "OUTPUT", "MODEL", "TITLE"
    );
    println!("{}", "-".repeat(80));
    for s in show {
        let uncertain = if s.usage.has_detailed_usage { "" } else { " ?" };
        println!(
            "{:>10}{:>10}{:>10}  {:<12}{}{}",
            cost_label(s),
            format_tokens(s.usage.input_tokens),
            format_tokens(s.usage.output_tokens),
            truncate(&model_short(&s.model_id), 12),
            truncate(&s.title, 40),
            uncertain
        );
    }
    if scan.sessions.len() > 25 {
        println!("… +{} more", scan.sessions.len() - 25);
    }
    println!();
    println!("cargo install --path .  · {PRICING_SOURCE}");
}

pub fn print_json(scan: &ScanResult) {
    let mut ordered = scan.sessions.clone();
    sort_sessions(&mut ordered, SortKey::Cost, true);

    let by_model: serde_json::Map<String, serde_json::Value> = scan
        .totals
        .by_model
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                serde_json::json!({
                    "input": v.input,
                    "output": v.output,
                    "cost": v.cost,
                    "sessions": v.sessions,
                }),
            )
        })
        .collect();

    let sessions: Vec<serde_json::Value> = ordered
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "title": s.title,
                "cwd": s.cwd,
                "modelId": s.model_id,
                "modelsUsed": s.models_used,
                "createdAt": s.created_at,
                "lastActiveAt": s.last_active_at,
                "turnCount": s.turn_count,
                "toolCallCount": s.tool_call_count,
                "contextTokensUsed": s.context_tokens_used,
                "inputTokens": s.usage.input_tokens,
                "outputTokens": s.usage.output_tokens,
                "streams": s.usage.streams,
                "hasDetailedUsage": s.usage.has_detailed_usage,
                "cost": s.usage.cost.total_cost,
                "inputCost": s.usage.cost.input_cost,
                "outputCost": s.usage.cost.output_cost,
                "priceModel": s.usage.cost.price.id,
                "byModel": s.usage.by_model.iter().map(|m| serde_json::json!({
                    "modelId": m.model_id,
                    "inputTokens": m.input_tokens,
                    "outputTokens": m.output_tokens,
                    "weight": m.weight,
                    "cost": m.cost.total_cost,
                })).collect::<Vec<_>>(),
                "isSubagent": s.is_subagent,
                "parentSessionId": s.parent_session_id,
                "childSessionIds": s.child_session_ids,
                "subagentType": s.subagent_type,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "scannedAt": scan.scanned_at,
        "grokHome": scan.grok_home,
        "sessionsDir": scan.sessions_dir,
        "pricingSource": PRICING_SOURCE,
        "totals": {
            "sessions": scan.totals.sessions,
            "withDetailed": scan.totals.with_detailed,
            "inputTokens": scan.totals.input_tokens,
            "outputTokens": scan.totals.output_tokens,
            "totalTokens": scan.totals.total_tokens,
            "totalCost": scan.totals.total_cost,
            "byModel": by_model,
        },
        "sessions": sessions,
    });

    println!("{}", serde_json::to_string_pretty(&payload).unwrap_or_default());
}

pub fn print_csv(scan: &ScanResult) {
    let mut ordered = scan.sessions.clone();
    sort_sessions(&mut ordered, SortKey::Cost, true);

    println!(
        "id,title,cwd,model,input_tokens,output_tokens,cost_usd,turns,tools,last_active,detailed,is_subagent"
    );
    for s in &ordered {
        let title = escape_csv(&s.title);
        let cwd = escape_csv(&s.cwd);
        println!(
            "{},{},{},{},{},{},{:.6},{},{},{},{},{}",
            s.id,
            title,
            cwd,
            s.model_id,
            s.usage.input_tokens,
            s.usage.output_tokens,
            s.usage.cost.total_cost,
            s.turn_count,
            s.tool_call_count,
            s.last_active_at.as_deref().unwrap_or(""),
            s.usage.has_detailed_usage,
            s.is_subagent,
        );
    }
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
