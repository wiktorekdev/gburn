use chrono::{DateTime, Local, Utc};

pub fn format_tokens(n: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    if n >= 1_000_000 {
        let v = n as f64 / 1_000_000.0;
        if n >= 10_000_000 {
            format!("{v:.1}M")
        } else {
            format!("{v:.2}M")
        }
    } else if n >= 1_000 {
        let v = n as f64 / 1_000.0;
        if n >= 100_000 {
            format!("{:.0}k", v)
        } else {
            format!("{v:.1}k")
        }
    } else {
        n.to_string()
    }
}

pub fn format_usd(n: f64) -> String {
    if !n.is_finite() {
        return "-".into();
    }
    if n == 0.0 {
        return "$0.00".into();
    }
    if n > 0.0 && n < 0.01 {
        return format!("${n:.4}");
    }
    format!("${n:.2}")
}

pub fn format_date(iso: Option<&str>) -> String {
    let Some(iso) = iso else {
        return "-".into();
    };
    match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string(),
        Err(_) => match chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.fZ") {
            Ok(naive) => {
                let dt = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
                dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string()
            }
            Err(_) => "-".into(),
        },
    }
}

pub fn format_relative(iso: Option<&str>) -> String {
    let Some(iso) = iso else {
        return "-".into();
    };
    let dt = match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => match chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.fZ") {
            Ok(naive) => DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc),
            Err(_) => return "-".into(),
        },
    };

    let now = Utc::now();
    let secs = (now - dt).num_seconds();
    if secs < 0 {
        return "now".into();
    }
    if secs < 60 {
        return format!("{secs}s ago");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = mins / 60;
    if hours < 48 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    if days < 14 {
        return format!("{days}d ago");
    }
    format_date(Some(iso))
}

pub fn format_duration(seconds: u64) -> String {
    if seconds == 0 {
        return "-".into();
    }
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "…".into();
    }
    s.chars().take(max - 1).collect::<String>() + "…"
}

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Right,
}

/// Truncate/pad by terminal display width (not char count).
pub fn fit_width(s: &str, width: usize, align: Align) -> String {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    if width == 0 {
        return String::new();
    }

    let full_w = s.width();
    let body = if full_w <= width {
        s.to_string()
    } else if width == 1 {
        "…".into()
    } else {
        let mut out = String::new();
        let mut used = 0usize;
        let budget = width.saturating_sub(1);
        for ch in s.chars() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + cw > budget {
                break;
            }
            out.push(ch);
            used += cw;
        }
        out.push('…');
        out
    };

    let used = body.width();
    let pad = width.saturating_sub(used);
    match align {
        Align::Left => format!("{body}{}", " ".repeat(pad)),
        Align::Right => format!("{}{body}", " ".repeat(pad)),
    }
}

pub fn short_path(p: &str, max: usize) -> String {
    if p.is_empty() {
        return "-".into();
    }
    let normalized = p.replace('/', "\\");
    if normalized.chars().count() <= max {
        return normalized;
    }
    let parts: Vec<&str> = normalized.split('\\').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 2 {
        return truncate(&normalized, max);
    }
    let tail = format!("…\\{}\\{}", parts[parts.len() - 2], parts[parts.len() - 1]);
    truncate(&tail, max)
}

pub fn decode_cwd_folder(name: &str) -> String {
    percent_encoding::percent_decode_str(name)
        .decode_utf8()
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| name.to_string())
}

pub fn model_short(id: &str) -> String {
    id.replace("grok-composer-2.5-fast", "composer")
        .replace("grok-4.5", "4.5")
        .replace("grok-build", "build")
        .replace("grok-", "")
}

pub fn spark_bar(ratio: f64, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let r = ratio.clamp(0.0, 1.0);
    let filled = (r * width as f64).round() as usize;
    let filled = filled.min(width);
    format!(
        "{}{}",
        "▓".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}

pub fn parse_iso_ms(iso: Option<&str>) -> i64 {
    let Some(iso) = iso else {
        return 0;
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return dt.timestamp_millis();
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.fZ") {
        return DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc).timestamp_millis();
    }
    0
}
