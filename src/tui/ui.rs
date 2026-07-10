use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::format::{
    fit_width, format_date, format_duration, format_relative, format_tokens, format_usd,
    model_short, short_path, spark_bar, truncate, Align,
};
use crate::pricing::{OFFICIAL_PRICES, PRICING_UPDATED};
use crate::scanner::SessionRecord;

use super::app::{App, ShowKind, View};
use super::hit::{ChipAction, ChipHit, OverallAction, OverallHit, RectHit, TabHit};

const BG: Color = Color::Rgb(12, 11, 10);
const ROW: Color = Color::Rgb(12, 11, 10);
const SEL: Color = Color::Rgb(36, 28, 20);
const LINE: Color = Color::Rgb(42, 38, 34);
const TEXT: Color = Color::Rgb(230, 224, 214);
const DIM: Color = Color::Rgb(110, 104, 96);
const MUTED: Color = Color::Rgb(150, 142, 132);
const EMBER: Color = Color::Rgb(255, 140, 48);
const FLAME: Color = Color::Rgb(255, 90, 40);
const COAL: Color = Color::Rgb(210, 70, 50);
const COOL: Color = Color::Rgb(130, 175, 165);
const GOLD: Color = Color::Rgb(255, 200, 110);
const INK: Color = Color::Rgb(18, 14, 10);
// bright filter chips (readable)
const CHIP_BG: Color = Color::Rgb(255, 176, 64);
const CHIP_FG: Color = Color::Rgb(22, 14, 8);
const CHIP_ALT_BG: Color = Color::Rgb(90, 120, 160);
const CHIP_ALT_FG: Color = Color::Rgb(245, 248, 255);

fn heat(amount: f64) -> Color {
    if amount >= 50.0 {
        COAL
    } else if amount >= 10.0 {
        FLAME
    } else if amount >= 2.0 {
        EMBER
    } else if amount > 0.0 {
        GOLD
    } else {
        DIM
    }
}


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

fn fill(f: &mut Frame, area: Rect, bg: Color) {
    f.render_widget(Block::default().style(Style::default().bg(bg)), area);
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    app.hits.clear();
    fill(f, area, BG);

    match app.view {
        View::Overall => draw_overall(f, app, area),
        View::List => draw_list(f, app, area),
        View::Detail => draw_detail(f, app, area),
        View::Pricing => draw_pricing(f, app, area),
        View::Models => draw_models(f, app, area),
        View::ModelDetail => draw_model_detail(f, app, area),
        View::Projects => draw_projects(f, app, area),
        View::ProjectDetail => draw_project_detail(f, app, area),
        View::Help => draw_help(f, app, area),
    }
}

fn zones(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

fn push_chip(app: &mut App, x: u16, y: u16, label: &str, action: ChipAction) -> u16 {
    let w = label.width() as u16;
    app.hits.chips.push(ChipHit {
        hit: RectHit::new(x, y, w, 1),
        action,
    });
    w
}

fn draw_header(f: &mut Frame, app: &mut App, area: Rect) {
    let totals = app.filtered_totals();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // on overall, header numbers ride the same count-up
    let ease = if app.view == View::Overall {
        app.reveal_eased()
    } else {
        1.0
    };
    let cost = totals.total_cost * ease;
    let tin = (totals.input_tokens as f64 * ease).round() as u64;
    let tout = (totals.output_tokens as f64 * ease).round() as u64;
    let rate = app.burn_per_day() * ease;
    let sess = anim_u(app.sessions.len(), ease);

    let stats = Line::from(vec![
        Span::styled(
            " gburn ",
            Style::default()
                .fg(INK)
                .bg(EMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}  ", format_usd(cost)),
            Style::default()
                .fg(EMBER)
                .bg(BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("in ", Style::default().fg(DIM).bg(BG)),
        Span::styled(
            format_tokens(tin),
            Style::default().fg(COOL).bg(BG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  out ", Style::default().fg(DIM).bg(BG)),
        Span::styled(
            format_tokens(tout),
            Style::default().fg(GOLD).bg(BG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ·  {sess} sess · {}/d", format_usd(rate)),
            Style::default().fg(DIM).bg(BG),
        ),
    ]);
    f.render_widget(Paragraph::new(stats).style(Style::default().bg(BG)), chunks[0]);

    // tabs + clickable chips
    let active = match app.view {
        View::Detail => View::List,
        View::ProjectDetail => View::Projects,
        View::ModelDetail => View::Models,
        other => other,
    };
    let tab_y = chunks[1].y;
    let mut x: u16 = chunks[1].x.saturating_add(1);
    let mut spans = vec![Span::styled(" ", Style::default().bg(BG))];

    let order = View::cycle_order();
    for (i, v) in order.iter().enumerate() {
        // show digit for keyboard: 1 overall, 2 sessions...
        let label = format!(" {} ", v.label());
        let w = label.width() as u16;
        app.hits.tabs.push(TabHit {
            hit: RectHit::new(x, tab_y, w, 1),
            view: *v,
        });
        if *v == active {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(INK)
                    .bg(EMBER)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(label, Style::default().fg(MUTED).bg(BG)));
        }
        x = x.saturating_add(w);
        if i + 1 < order.len() {
            spans.push(Span::styled("·", Style::default().fg(LINE).bg(BG)));
            x = x.saturating_add(1);
        }
    }

    spans.push(Span::styled("  ", Style::default().bg(BG)));
    x = x.saturating_add(2);

    let time_label = format!(" {} ", app.time_range.label());
    let tw = push_chip(app, x, tab_y, &time_label, ChipAction::TimeRange);
    spans.push(Span::styled(
        time_label.clone(),
        Style::default()
            .fg(CHIP_FG)
            .bg(CHIP_BG)
            .add_modifier(Modifier::BOLD),
    ));
    x = x.saturating_add(tw + 1);
    spans.push(Span::styled(" ", Style::default().bg(BG)));

    let sort_label = app.sort_chip_label();
    let sw = push_chip(app, x, tab_y, &sort_label, ChipAction::Sort);
    spans.push(Span::styled(
        sort_label.clone(),
        Style::default()
            .fg(CHIP_ALT_FG)
            .bg(CHIP_ALT_BG)
            .add_modifier(Modifier::BOLD),
    ));
    x = x.saturating_add(sw + 1);
    spans.push(Span::styled(" ", Style::default().bg(BG)));

    let find_label = " / ".to_string();
    let _ = push_chip(app, x, tab_y, &find_label, ChipAction::Search);
    spans.push(Span::styled(
        find_label,
        Style::default()
            .fg(CHIP_FG)
            .bg(CHIP_BG)
            .add_modifier(Modifier::BOLD),
    ));

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(BG)),
        chunks[1],
    );
}

fn draw_footer(f: &mut Frame, app: &mut App, area: Rect) {
    fill(f, area, BG);
    app.hits.footer_meta = Some(RectHit::from_ratatui(area));

    if app.searching {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" find: ", Style::default().fg(CHIP_BG).bg(BG)),
                Span::styled(
                    format!("{}_", app.query),
                    Style::default().fg(TEXT).bg(BG).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  enter  esc", Style::default().fg(DIM).bg(BG)),
            ]))
            .style(Style::default().bg(BG)),
            area,
        );
        return;
    }

    let sel_line = match app.view {
        View::Projects | View::ProjectDetail => {
            if let Some(p) = app.current_project() {
                Line::from(vec![
                    Span::styled(" ", Style::default().bg(BG)),
                    Span::styled(
                        format_usd(p.total_cost),
                        Style::default().fg(heat(p.total_cost)).bg(BG),
                    ),
                    Span::styled(
                        format!("  {} sess  ", p.sessions),
                        Style::default().fg(DIM).bg(BG),
                    ),
                    Span::styled(short_path(&p.cwd, 48), Style::default().fg(MUTED).bg(BG)),
                ])
            } else {
                Line::from(Span::styled(" no project", Style::default().fg(DIM).bg(BG)))
            }
        }
        View::Models | View::ModelDetail => {
            if let Some((id, row)) = app.current_model() {
                Line::from(vec![
                    Span::styled(" ", Style::default().bg(BG)),
                    Span::styled(
                        format_usd(row.cost),
                        Style::default().fg(heat(row.cost)).bg(BG),
                    ),
                    Span::styled("  ", Style::default().bg(BG)),
                    Span::styled(
                        format_tokens(row.input),
                        Style::default().fg(COOL).bg(BG),
                    ),
                    Span::styled(" in  ", Style::default().fg(DIM).bg(BG)),
                    Span::styled(
                        format_tokens(row.output),
                        Style::default().fg(GOLD).bg(BG),
                    ),
                    Span::styled(" out  ", Style::default().fg(DIM).bg(BG)),
                    Span::styled(model_short(&id), Style::default().fg(MUTED).bg(BG)),
                ])
            } else {
                Line::from(Span::styled(" no model", Style::default().fg(DIM).bg(BG)))
            }
        }
        View::Pricing | View::Help => Line::from(Span::styled(
            " ",
            Style::default().fg(DIM).bg(BG),
        )),
        View::Overall => {
            let t = app.filtered_totals();
            Line::from(vec![
                Span::styled(" ", Style::default().bg(BG)),
                Span::styled(
                    format_usd(t.total_cost),
                    Style::default().fg(EMBER).bg(BG),
                ),
                Span::styled(
                    format!("  {} sessions", t.sessions),
                    Style::default().fg(DIM).bg(BG),
                ),
            ])
        }
        _ => {
            if let Some(s) = app.selected_session() {
                Line::from(vec![
                    Span::styled(" ", Style::default().bg(BG)),
                    Span::styled(
                        format_tokens(s.usage.input_tokens),
                        Style::default().fg(COOL).bg(BG),
                    ),
                    Span::styled(" in  ", Style::default().fg(DIM).bg(BG)),
                    Span::styled(
                        format_tokens(s.usage.output_tokens),
                        Style::default().fg(GOLD).bg(BG),
                    ),
                    Span::styled(" out  ", Style::default().fg(DIM).bg(BG)),
                    Span::styled(model_short(&s.model_id), Style::default().fg(MUTED).bg(BG)),
                    Span::styled("  ", Style::default().bg(BG)),
                    Span::styled(short_path(&s.cwd, 48), Style::default().fg(DIM).bg(BG)),
                ])
            } else {
                Line::from(Span::styled(" no selection", Style::default().fg(DIM).bg(BG)))
            }
        }
    };
    f.render_widget(Paragraph::new(sel_line).style(Style::default().bg(BG)), area);
}

fn draw_overall(f: &mut Frame, app: &mut App, area: Rect) {
    let (header, body, footer) = zones(area);
    draw_header(f, app, header);
    draw_footer(f, app, footer);

    let totals = app.filtered_totals();
    let ease = app.reveal_eased();
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(body);

    draw_show_scene(f, app, cols[0], &totals, ease);
    draw_overall_stats(f, app, cols[1], &totals, ease);
}


fn anim_usd(target: f64, ease: f64) -> String {
    format_usd(target * ease)
}

fn anim_u(target: usize, ease: f64) -> usize {
    ((target as f64) * ease).round() as usize
}

fn draw_show_scene(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    totals: &crate::scanner::ScanTotals,
    ease: f64,
) {
    fill(f, area, BG);
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(LINE))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let h = inner.height as usize;
    let w = inner.width as usize;
    if h < 8 || w < 14 {
        return;
    }

    match app.show_kind {
        ShowKind::Rocket => draw_rocket_show(f, inner, app, totals, ease, h, w),
        ShowKind::Casino => draw_casino_show(f, inner, app, totals, ease, h, w),
        ShowKind::Inferno => draw_inferno_show(f, inner, app, totals, ease, h, w),
    }
}

fn draw_rocket_show(
    f: &mut Frame,
    area: Rect,
    app: &App,
    totals: &crate::scanner::ScanTotals,
    ease: f64,
    h: usize,
    w: usize,
) {
    let frame = app.anim_frame;
    let t = app.reveal.clamp(0.0, 1.0);
    let done = app.show_done || t >= 1.0;
    let mut lines: Vec<String> = (0..h).map(|_| " ".repeat(w)).collect();
    let cx = w / 2;

    // stars
    for i in 0..h {
        for j in 0..(w / 4).max(1) {
            let seed = (i * 31 + j * 17 + frame as usize / 8) % 50;
            if seed < 4 {
                let x = (i * 13 + j * 29 + seed * 7) % w;
                put_char(&mut lines[i], x, if seed == 0 { '*' } else { '.' });
            }
        }
    }

    let rocket = [
        r"   /^\   ",
        r"  //|\\  ",
        r" |=====| ",
        r" | GBR | ",
        r" |=====| ",
        r"  /|||\  ",
        r" //|||\\ ",
    ];
    let rh = rocket.len();
    let pad_y = h.saturating_sub(rh + 3);
    let fly = if done {
        pad_y
    } else if t < 0.16 {
        0
    } else {
        let u = ((t - 0.16) / 0.72).clamp(0.0, 1.0);
        ((1.0 - (1.0 - u).powi(2)) * pad_y as f64).round() as usize
    };
    let rocket_y = pad_y.saturating_sub(fly.min(pad_y));
    let shake = if t > 0.14 && t < 0.28 {
        (frame % 3) as i32 - 1
    } else {
        0
    };
    let rcx = ((cx as i32) + shake).clamp(4, w as i32 - 5) as usize;

    // pad
    let gy = h.saturating_sub(1);
    lines[gy] = "=".repeat(w);
    if gy > 0 {
        blit_center(&mut lines[gy - 1], cx, "|=|");
    }

    // smoke near pad early
    if !done && t < 0.55 && gy > 1 {
        for k in 0..4 {
            let ox = ((frame as i32 / 2 + k * 2) % 5) - 2;
            let x = (rcx as i32 + ox).clamp(0, w as i32 - 1) as usize;
            put_char(&mut lines[gy - 1], x, if k % 2 == 0 { '~' } else { '.' });
        }
    }

    // plume
    let plume_n = if t < 0.14 { 1 } else if t < 0.35 { 7 } else { 5 };
    for p in 0..plume_n {
        let py = rocket_y + rh + p;
        if py >= gy {
            break;
        }
        let width = 1 + p.min(3);
        let wob = ((frame as i32 + p as i32) % 3) - 1;
        for dx in 0..=width {
            let x = (rcx as i32 + wob + dx as i32 - width as i32 / 2)
                .clamp(0, w as i32 - 1) as usize;
            let ch = match (frame + p as u64) % 4 {
                0 => '#',
                1 => '*',
                2 => '+',
                _ => '.',
            };
            put_char(&mut lines[py], x, ch);
        }
    }

    for (i, spr) in rocket.iter().enumerate() {
        let y = rocket_y + i;
        if y < h {
            blit_center(&mut lines[y], rcx, spr);
        }
    }

    let money = format!(" {} ", format_usd(totals.total_cost * ease));
    if h > 2 {
        blit_center(&mut lines[h.saturating_sub(3)], cx, &money);
    }

    let styled: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(yi, row)| {
            let fg = if yi >= rocket_y && yi < rocket_y + rh {
                TEXT
            } else if yi >= rocket_y + rh && yi < rocket_y + rh + plume_n {
                FLAME
            } else if yi == h.saturating_sub(3) {
                GOLD
            } else if yi >= gy.saturating_sub(1) {
                MUTED
            } else {
                DIM
            };
            Line::from(Span::styled(
                row,
                Style::default().fg(fg).bg(BG).add_modifier(
                    if yi >= rocket_y && yi < rocket_y + rh {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    },
                ),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(styled).style(Style::default().bg(BG)), area);
}

fn draw_casino_show(
    f: &mut Frame,
    area: Rect,
    app: &App,
    totals: &crate::scanner::ScanTotals,
    _ease: f64,
    h: usize,
    w: usize,
) {
    let frame = app.anim_frame;
    let t = app.reveal.clamp(0.0, 1.0);
    let done = app.show_done || t >= 1.0;
    let mut lines: Vec<String> = (0..h).map(|_| " ".repeat(w)).collect();
    let cx = w / 2;

    let target = format_usd(totals.total_cost);
    let target_chars: Vec<char> = target.chars().collect();
    let n = target_chars.len().max(1);

    let mut display = Vec::with_capacity(n);
    for (i, &final_ch) in target_chars.iter().enumerate() {
        let lock_t = 0.30 + (i as f64 + 1.0) / (n as f64 + 1.0) * 0.58;
        if done || t >= lock_t {
            display.push(final_ch);
        } else {
            display.push(spin_glyph(frame, i, final_ch));
        }
    }

    let mid = h / 2;
    let box_w = (n * 3 + 4).min(w.saturating_sub(2)).max(10);
    let top = format!("+{}+", "-".repeat(box_w.saturating_sub(2)));
    let bot = format!("+{}+", "-".repeat(box_w.saturating_sub(2)));

    if mid >= 2 {
        blit_center(&mut lines[mid - 2], cx, &top);
    }

    let mut reel = String::from("|");
    for ch in &display {
        reel.push(' ');
        reel.push(*ch);
        reel.push(' ');
    }
    reel.push('|');
    if reel.chars().count() > w {
        reel = reel.chars().take(w).collect();
    }
    blit_center(&mut lines[mid], cx, &reel);

    if !done && mid > 0 {
        let mut blur = String::from("|");
        for i in 0..n {
            blur.push(' ');
            blur.push(spin_glyph(frame + 2, i, '0'));
            blur.push(' ');
        }
        blur.push('|');
        blit_center(&mut lines[mid - 1], cx, &blur);
        if mid + 1 < h {
            let mut blur2 = String::from("|");
            for i in 0..n {
                blur2.push(' ');
                blur2.push(spin_glyph(frame + 5, i + 1, '0'));
                blur2.push(' ');
            }
            blur2.push('|');
            blit_center(&mut lines[mid + 1], cx, &blur2);
        }
    }

    if mid + 2 < h {
        blit_center(&mut lines[mid + 2], cx, &bot);
    }

    if done {
        if mid + 4 < h {
            blit_center(&mut lines[mid + 4], cx, " 7  7  7 ");
        }
        for i in 1..h.saturating_sub(1) {
            if (i + frame as usize) % 6 == 0 {
                put_char(&mut lines[i], 1, '·');
                put_char(&mut lines[i], w.saturating_sub(2), '·');
            }
        }
    }

    let styled: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(yi, row)| {
            let fg = if yi == mid {
                if done { GOLD } else { TEXT }
            } else if yi.abs_diff(mid) <= 2 {
                EMBER
            } else if done {
                FLAME
            } else {
                DIM
            };
            Line::from(Span::styled(
                row,
                Style::default().fg(fg).bg(BG).add_modifier(if yi == mid {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(styled).style(Style::default().bg(BG)), area);
}

fn spin_glyph(frame: u64, reel: usize, prefer: char) -> char {
    if matches!(prefer, '$' | '.') && (frame / 2 + reel as u64) % 4 == 0 {
        return prefer;
    }
    const POOL: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '7', '$'];
    if (frame + reel as u64) % 8 == 0 {
        '7'
    } else {
        POOL[((frame as usize * 7) + reel * 13) % POOL.len()]
    }
}

fn draw_inferno_show(
    f: &mut Frame,
    area: Rect,
    app: &App,
    totals: &crate::scanner::ScanTotals,
    ease: f64,
    h: usize,
    w: usize,
) {
    let frame = app.anim_frame;
    let t = app.reveal.clamp(0.0, 1.0);
    let done = app.show_done || t >= 1.0;
    let mut lines: Vec<String> = (0..h).map(|_| " ".repeat(w)).collect();
    let cx = w / 2;

    // fire columns rising with ease
    let cols = (w / 3).max(6).min(24);
    let col_w = w / cols.max(1);
    for c in 0..cols {
        let height = if done {
            h.saturating_sub(3)
        } else {
            let jitter = ((frame as usize + c * 5) % 4) as f64 * 0.04;
            (((ease + jitter).min(1.0)) * (h.saturating_sub(3)) as f64).round() as usize
        };
        let x = c * col_w + col_w / 2;
        for row in 0..height {
            let y = h.saturating_sub(2).saturating_sub(row);
            if y >= h || x >= w {
                continue;
            }
            let ch = match (frame as usize + row + c) % 5 {
                0 => '#',
                1 => '*',
                2 => '%',
                3 => '&',
                _ => '^',
            };
            put_char(&mut lines[y], x, ch);
            if x + 1 < w && row > 0 {
                put_char(&mut lines[y], x + 1, if row % 2 == 0 { '*' } else { ' ' });
            }
        }
    }

    // ground
    if h > 0 {
        lines[h - 1] = "=".repeat(w);
    }

    // big total
    let money = format!(" {} ", format_usd(totals.total_cost * ease));
    let my = h / 2;
    blit_center(&mut lines[my], cx, &money);
    if my > 0 {
        blit_center(
            &mut lines[my.saturating_sub(1)],
            cx,
            if done { "  BURNED  " } else { "  BURNING  " },
        );
    }

    // for huge amounts, flash ring
    if done && totals.total_cost >= 1000.0 {
        for i in 0..h {
            if (i + frame as usize) % 5 == 0 {
                put_char(&mut lines[i], 0, '>');
                put_char(&mut lines[i], w.saturating_sub(1), '<');
            }
        }
    }

    let styled: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(yi, row)| {
            let fg = if yi == my {
                GOLD
            } else if yi == my.saturating_sub(1) {
                FLAME
            } else if yi == h.saturating_sub(1) {
                COAL
            } else {
                EMBER
            };
            Line::from(Span::styled(
                row,
                Style::default().fg(fg).bg(BG).add_modifier(if yi == my {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(styled).style(Style::default().bg(BG)), area);
}

fn blit_center(line: &mut String, cx: usize, text: &str) {
    let chars: Vec<char> = text.chars().collect();
    let start = cx.saturating_sub(chars.len() / 2);
    let mut buf: Vec<char> = line.chars().collect();
    for (i, ch) in chars.into_iter().enumerate() {
        let x = start + i;
        if x < buf.len() {
            buf[x] = ch;
        }
    }
    *line = buf.into_iter().collect();
}

fn put_char(line: &mut String, x: usize, ch: char) {
    let mut chars: Vec<char> = line.chars().collect();
    if x < chars.len() {
        chars[x] = ch;
        *line = chars.into_iter().collect();
    }
}

fn push_overall_hit(app: &mut App, area: Rect, row: u16, action: OverallAction) {
    app.hits.overall.push(OverallHit {
        hit: RectHit::new(area.x, area.y + row, area.width, 1),
        action,
    });
}

fn draw_overall_stats(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    totals: &crate::scanner::ScanTotals,
    ease: f64,
) {
    fill(f, area, BG);

    let input_n = (totals.input_tokens as f64 * ease).round() as u64;
    let output_n = (totals.output_tokens as f64 * ease).round() as u64;
    let rate = app.burn_per_day() * ease;
    let range_label = app.time_range.label();

    let burners: Vec<(f64, String)> = app
        .top_burners(5)
        .into_iter()
        .map(|s| (s.usage.cost.total_cost, s.title.clone()))
        .collect();
    let show_n = if ease < 0.4 {
        0
    } else {
        (((ease - 0.4) / 0.6) * burners.len() as f64).ceil() as usize
    }
    .min(burners.len());

    let mut models: Vec<(String, f64)> = totals
        .by_model
        .iter()
        .map(|(id, row)| (id.clone(), row.cost))
        .collect();
    models.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut lines: Vec<Line> = Vec::new();
    let mut hit_rows: Vec<(u16, OverallAction)> = Vec::new();

    lines.push(Line::from(Span::styled(
        format!("  {}", anim_usd(totals.total_cost, ease)),
        Style::default().fg(EMBER).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(format_tokens(input_n), Style::default().fg(COOL)),
        Span::styled(" in  ", Style::default().fg(DIM)),
        Span::styled(format_tokens(output_n), Style::default().fg(GOLD)),
        Span::styled(" out", Style::default().fg(DIM)),
    ]));
    lines.push(Line::from(""));

    let sess_row = lines.len() as u16;
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {} sessions", anim_u(totals.sessions, ease)),
            Style::default().fg(TEXT),
        ),
        Span::styled(
            format!("  ·  {}/d", format_usd(rate)),
            Style::default().fg(DIM),
        ),
    ]));
    hit_rows.push((sess_row, OverallAction::Sessions));

    let range_row = lines.len() as u16;
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            format!(" {range_label} "),
            Style::default()
                .fg(CHIP_FG)
                .bg(CHIP_BG)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    hit_rows.push((range_row, OverallAction::Range));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  top",
        Style::default().fg(DIM),
    )));

    for (i, (cost, title)) in burners.iter().enumerate().take(show_n) {
        let row = lines.len() as u16;
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:>8}  ", format_usd(cost * ease)),
                Style::default().fg(heat(*cost)),
            ),
            Span::styled(truncate(title, 34), Style::default().fg(TEXT)),
        ]));
        hit_rows.push((row, OverallAction::Burner(i)));
    }

    lines.push(Line::from(""));
    let models_row = lines.len() as u16;
    lines.push(Line::from(Span::styled(
        "  models",
        Style::default().fg(DIM),
    )));
    hit_rows.push((models_row, OverallAction::Models));

    for (id, cost) in models.into_iter().take(3) {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<10}", truncate(&model_short(&id), 10)),
                Style::default().fg(MUTED),
            ),
            Span::styled(anim_usd(cost, ease), Style::default().fg(heat(cost))),
        ]));
    }

    for (row, action) in hit_rows {
        push_overall_hit(app, area, row, action);
    }

    f.render_widget(Paragraph::new(lines).style(Style::default().bg(BG)), area);
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let (header, body, footer) = zones(area);
    draw_header(f, app, header);
    draw_footer(f, app, footer);

    app.list_height = body.height.max(1) as usize;
    app.hits.list = Some(RectHit::from_ratatui(body));
    app.ensure_visible();

    fill(f, body, BG);

    if app.sessions.is_empty() {
        f.render_widget(
            Paragraph::new("  no sessions")
                .style(Style::default().fg(DIM).bg(BG)),
            body,
        );
        return;
    }

    let max_cost = app
        .sessions
        .iter()
        .map(|s| s.usage.cost.total_cost)
        .fold(0.01_f64, f64::max);

    let end = (app.list_scroll + app.list_height).min(app.sessions.len());
    let w = body.width as usize;

    // fixed columns: caret(1) cost(9) gap(1) bar(5) gap(1) title(flex) gap(1) when(9)
    const COST_W: usize = 9;
    const BAR_W: usize = 5;
    const WHEN_W: usize = 9;
    let fixed = 1 + COST_W + 1 + BAR_W + 1 + 1 + WHEN_W;
    let title_w = w.saturating_sub(fixed).max(8);

    for (row_i, idx) in (app.list_scroll..end).enumerate() {
        let s = &app.sessions[idx];
        let selected = idx == app.selected;
        let bg = if selected { SEL } else { ROW };
        let cost = s.usage.cost.total_cost;
        let hc = heat(cost);
        let ratio = cost / max_cost;

        let mut title = String::new();
        if s.is_subagent {
            title.push_str("> ");
        }
        title.push_str(&s.title);
        if !s.child_session_ids.is_empty() {
            title.push_str(&format!(" (+{})", s.child_session_ids.len()));
        }

        let when = fit_width(
            &format_relative(s.last_active_at.as_deref()),
            WHEN_W,
            Align::Right,
        );
        let cost_s = fit_width(&cost_label(s), COST_W, Align::Right);
        let title_s = fit_width(&title, title_w, Align::Left);
        let bar = spark_bar(ratio, BAR_W);

        let row_rect = Rect {
            x: body.x,
            y: body.y + row_i as u16,
            width: body.width,
            height: 1,
        };
        // full-row background so selection never leaves gaps
        fill(f, row_rect, bg);

        let line = Line::from(vec![
            Span::styled(
                if selected { "▌" } else { " " },
                Style::default().fg(EMBER).bg(bg),
            ),
            Span::styled(
                cost_s,
                Style::default()
                    .fg(if selected { INK } else { hc })
                    .bg(if selected { hc } else { bg })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(bar, Style::default().fg(hc).bg(bg)),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(
                title_s,
                Style::default()
                    .fg(TEXT)
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(when, Style::default().fg(DIM).bg(bg)),
        ]);

        f.render_widget(Paragraph::new(line).style(Style::default().bg(bg)), row_rect);
    }
}

fn page(f: &mut Frame, app: &mut App, area: Rect, title: &str, lines: Vec<Line>) {
    let (header, body, footer) = zones(area);
    draw_header(f, app, header);
    draw_footer(f, app, footer);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(LINE))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(EMBER).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG));

    let inner = block.inner(body);
    // scroll budget: lines that overflow the inner height
    let visible = inner.height.max(1);
    let max_scroll = (lines.len() as u16).saturating_sub(visible);
    app.detail_max_scroll = max_scroll;
    if app.detail_scroll > max_scroll {
        app.detail_scroll = max_scroll;
    }
    app.hits.page_body = Some(RectHit::from_ratatui(inner));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(TEXT).bg(BG))
            .scroll((app.detail_scroll, 0))
            .wrap(Wrap { trim: false }),
        body,
    );
}

fn draw_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(s) = app.selected_session().cloned() else {
        page(
            f,
            app,
            area,
            "detail",
            vec![Line::from(Span::styled(
                "  nothing selected",
                Style::default().fg(DIM),
            ))],
        );
        return;
    };

    let u = &s.usage;
    let mut lines = vec![
        Line::from(Span::styled(
            format!("  {}", s.title),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("  {}", short_path(&s.cwd, 70)),
            Style::default().fg(DIM),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  cost   ", Style::default().fg(DIM)),
            Span::styled(
                format_usd(u.cost.total_cost),
                Style::default()
                    .fg(heat(u.cost.total_cost))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "   (in {} · out {})",
                    format_usd(u.cost.input_cost),
                    format_usd(u.cost.output_cost)
                ),
                Style::default().fg(MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("  tokens ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "{} in · {} out",
                    format_tokens(u.input_tokens),
                    format_tokens(u.output_tokens)
                ),
                Style::default().fg(TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  model  ", Style::default().fg(DIM)),
            Span::styled(s.model_id.clone(), Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  when   ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "{} · {}",
                    format_date(s.last_active_at.as_deref()),
                    format_duration(s.session_duration_seconds)
                ),
                Style::default().fg(TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  work   ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "{} turns · {} tools · {} msgs",
                    s.turn_count, s.tool_call_count, s.num_messages
                ),
                Style::default().fg(TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  rate   ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "${} / ${} per 1M · {}",
                    u.cost.price.input_per_m,
                    u.cost.price.output_per_m,
                    if u.has_detailed_usage {
                        "detailed"
                    } else {
                        "approx"
                    }
                ),
                Style::default().fg(MUTED),
            ),
        ]),
    ];

    if s.is_subagent {
        lines.push(Line::from(vec![
            Span::styled("  sub    ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "{} · parent {}",
                    s.subagent_type.as_deref().unwrap_or("?"),
                    s.parent_session_id
                        .as_deref()
                        .map(|p| truncate(p, 8))
                        .unwrap_or_else(|| "-".into())
                ),
                Style::default().fg(TEXT),
            ),
        ]));
        if let Some(desc) = &s.subagent_description {
            lines.push(Line::from(vec![
                Span::styled("  task   ", Style::default().fg(DIM)),
                Span::styled(truncate(desc, 60), Style::default().fg(MUTED)),
            ]));
        }
    } else if !s.child_session_ids.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  kids   ", Style::default().fg(DIM)),
            Span::styled(
                format!("{} subagents", s.child_session_ids.len()),
                Style::default().fg(TEXT),
            ),
        ]));
    }

    if u.by_model.len() > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  models",
            Style::default().fg(DIM),
        )));
        for m in &u.by_model {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {:<12}", truncate(&model_short(&m.model_id), 12)),
                    Style::default().fg(MUTED),
                ),
                Span::styled(
                    format!(
                        "{} / {}  {}",
                        format_tokens(m.input_tokens),
                        format_tokens(m.output_tokens),
                        format_usd(m.cost.total_cost)
                    ),
                    Style::default().fg(TEXT),
                ),
            ]));
        }
    }

    if !u.streams_detail.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  top streams",
            Style::default().fg(DIM),
        )));
        let mut top = u.streams_detail.clone();
        top.sort_by(|a, b| b.input_tokens.cmp(&a.input_tokens));
        top.truncate(8);
        let max_in = top.iter().map(|x| x.input_tokens).max().unwrap_or(1).max(1);
        for st in &top {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "    {:>7} in  {:>7} out  ",
                        format_tokens(st.input_tokens),
                        format_tokens(st.output_tokens)
                    ),
                    Style::default().fg(MUTED),
                ),
                Span::styled(
                    spark_bar(st.input_tokens as f64 / max_in as f64, 12),
                    Style::default().fg(EMBER),
                ),
            ]));
        }
    }

    page(f, app, area, "session", lines);
}

fn draw_pricing(f: &mut Frame, app: &mut App, area: Rect) {
    let (header, body, footer) = zones(area);
    draw_header(f, app, header);
    draw_footer(f, app, footer);
    fill(f, body, BG);

    let max_out = OFFICIAL_PRICES
        .iter()
        .filter(|p| p.id != "grok-build-0.1")
        .map(|p| p.output_per_m)
        .fold(0.01_f64, f64::max);

    let mut y = 0u16;
    let paint = |f: &mut Frame, y: u16, line: Line| {
        if y >= body.height {
            return;
        }
        f.render_widget(
            Paragraph::new(line).style(Style::default().bg(BG)),
            Rect {
                x: body.x,
                y: body.y + y,
                width: body.width,
                height: 1,
            },
        );
    };

    paint(
        f,
        y,
        Line::from(Span::styled(
            format!("  rates  {PRICING_UPDATED}"),
            Style::default().fg(DIM),
        )),
    );
    y += 1;
    paint(f, y, Line::from(""));
    y += 1;

    for p in OFFICIAL_PRICES {
        if p.id == "grok-build-0.1" {
            continue;
        }
        if y + 2 >= body.height {
            break;
        }
        let bar = spark_bar(p.output_per_m / max_out, 14);
        paint(
            f,
            y,
            Line::from(vec![
                Span::styled(
                    format!("  {:<20}", p.label),
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ctx {}", p.context.unwrap_or("-")),
                    Style::default().fg(DIM),
                ),
            ]),
        );
        y += 1;
        paint(
            f,
            y,
            Line::from(vec![
                Span::styled("    in ", Style::default().fg(DIM)),
                Span::styled(
                    format!("${:<6.2}", p.input_per_m),
                    Style::default().fg(COOL).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" out ", Style::default().fg(DIM)),
                Span::styled(
                    format!("${:<6.2}", p.output_per_m),
                    Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {bar}"), Style::default().fg(EMBER)),
            ]),
        );
        y += 1;
        paint(f, y, Line::from(""));
        y += 1;
    }
}

fn draw_nav_rows(
    f: &mut Frame,
    body: Rect,
    rows: Vec<(bool, Line<'static>)>,
    scroll: usize,
    list_h: usize,
) {
    fill(f, body, BG);
    let end = (scroll + list_h).min(rows.len());
    for (i, idx) in (scroll..end).enumerate() {
        let (selected, line) = &rows[idx];
        let bg = if *selected { SEL } else { BG };
        let rect = Rect {
            x: body.x,
            y: body.y + i as u16,
            width: body.width,
            height: 1,
        };
        fill(f, rect, bg);
        f.render_widget(Paragraph::new(line.clone()).style(Style::default().bg(bg)), rect);
    }
}

fn draw_models(f: &mut Frame, app: &mut App, area: Rect) {
    let (header, body, footer) = zones(area);
    draw_header(f, app, header);
    draw_footer(f, app, footer);

    let rows_data = app.model_rows();
    app.list_height = body.height.max(1) as usize;
    app.hits.list = Some(RectHit::from_ratatui(body));
    app.ensure_model_visible();

    fill(f, body, BG);

    if rows_data.is_empty() {
        f.render_widget(
            Paragraph::new("  no models").style(Style::default().fg(DIM).bg(BG)),
            body,
        );
        return;
    }

    let max = rows_data
        .iter()
        .map(|(_, r)| r.cost)
        .fold(0.01_f64, f64::max);

    // fixed columns: caret name cost in out bar  (labels inside in/out widths)
    const NAME_W: usize = 18;
    const COST_W: usize = 9;
    const IN_W: usize = 11;
    const OUT_W: usize = 12;
    const BAR_W: usize = 10;

    let end = (app.model_scroll + app.list_height).min(rows_data.len());
    for (row_i, idx) in (app.model_scroll..end).enumerate() {
        let (model, row) = &rows_data[idx];
        let selected = idx == app.selected_model;
        let bg = if selected { SEL } else { BG };
        let hc = heat(row.cost);

        let name = fit_width(model, NAME_W, Align::Left);
        let cost = fit_width(&format_usd(row.cost), COST_W, Align::Right);
        let tin = fit_width(
            &format!("{} in", format_tokens(row.input)),
            IN_W,
            Align::Right,
        );
        let tout = fit_width(
            &format!("{} out", format_tokens(row.output)),
            OUT_W,
            Align::Right,
        );
        let bar = spark_bar(row.cost / max, BAR_W);

        let row_rect = Rect {
            x: body.x,
            y: body.y + row_i as u16,
            width: body.width,
            height: 1,
        };
        fill(f, row_rect, bg);

        let line = Line::from(vec![
            Span::styled(
                if selected { "▌" } else { " " },
                Style::default().fg(EMBER).bg(bg),
            ),
            Span::styled(name, Style::default().fg(TEXT).bg(bg)),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(
                cost,
                Style::default()
                    .fg(if selected { INK } else { hc })
                    .bg(if selected { hc } else { bg })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(
                tin,
                Style::default().fg(if selected { MUTED } else { COOL }).bg(bg),
            ),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(
                tout,
                Style::default().fg(if selected { MUTED } else { GOLD }).bg(bg),
            ),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(bar, Style::default().fg(hc).bg(bg)),
        ]);
        f.render_widget(Paragraph::new(line).style(Style::default().bg(bg)), row_rect);
    }
}

fn draw_projects(f: &mut Frame, app: &mut App, area: Rect) {
    let (header, body, footer) = zones(area);
    draw_header(f, app, header);
    draw_footer(f, app, footer);

    let projects = app.projects();
    app.list_height = body.height.max(1) as usize;
    app.hits.list = Some(RectHit::from_ratatui(body));
    app.ensure_project_visible();

    let max = projects
        .iter()
        .map(|p| p.total_cost)
        .fold(0.01_f64, f64::max);

    let mut rows = Vec::new();
    for (i, p) in projects.iter().enumerate() {
        let selected = i == app.selected_project;
        let bg_hint = if selected { SEL } else { BG };
        let caret = if selected { "▌" } else { " " };
        let line = Line::from(vec![
            Span::styled(caret, Style::default().fg(EMBER).bg(bg_hint)),
            Span::styled(
                format!(" {:<34}", short_path(&p.cwd, 34)),
                Style::default().fg(TEXT).bg(bg_hint),
            ),
            Span::styled(
                format!("{:>9}", format_usd(p.total_cost)),
                Style::default()
                    .fg(heat(p.total_cost))
                    .bg(bg_hint)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  {:>3} sess  {}",
                    p.sessions,
                    spark_bar(p.total_cost / max, 10)
                ),
                Style::default().fg(DIM).bg(bg_hint),
            ),
        ]);
        rows.push((selected, line));
    }
    if rows.is_empty() {
        fill(f, body, BG);
        f.render_widget(
            Paragraph::new("  no projects").style(Style::default().fg(DIM).bg(BG)),
            body,
        );
        return;
    }
    draw_nav_rows(f, body, rows, app.project_scroll, app.list_height);
}

fn draw_project_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(p) = app.current_project() else {
        page(
            f,
            app,
            area,
            "project",
            vec![Line::from(Span::styled(
                "  nothing selected",
                Style::default().fg(DIM),
            ))],
        );
        return;
    };
    let sess = app.sessions_for_project(&p.cwd);
    let mut lines = vec![
        Line::from(Span::styled(
            format!("  {}", short_path(&p.cwd, 64)),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  cost     ", Style::default().fg(DIM)),
            Span::styled(
                format_usd(p.total_cost),
                Style::default().fg(heat(p.total_cost)).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  tokens   ", Style::default().fg(DIM)),
            Span::styled(format_tokens(p.input_tokens), Style::default().fg(COOL)),
            Span::styled(" in  ", Style::default().fg(DIM)),
            Span::styled(format_tokens(p.output_tokens), Style::default().fg(GOLD)),
            Span::styled(" out", Style::default().fg(DIM)),
        ]),
        Line::from(vec![
            Span::styled("  sessions ", Style::default().fg(DIM)),
            Span::styled(format!("{}", p.sessions), Style::default().fg(TEXT)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  sessions", Style::default().fg(DIM))),
    ];
    for s in sess.iter().take(20) {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:>8}  ", format_usd(s.usage.cost.total_cost)),
                Style::default().fg(heat(s.usage.cost.total_cost)),
            ),
            Span::styled(truncate(&s.title, 40), Style::default().fg(TEXT)),
        ]));
    }
    if sess.len() > 20 {
        lines.push(Line::from(Span::styled(
            format!("  +{} more", sess.len() - 20),
            Style::default().fg(DIM),
        )));
    }
    page(f, app, area, "project", lines);
}

fn draw_model_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some((model_id, row)) = app.current_model() else {
        page(
            f,
            app,
            area,
            "model",
            vec![Line::from(Span::styled(
                "  nothing selected",
                Style::default().fg(DIM),
            ))],
        );
        return;
    };
    let sess = app.sessions_for_model(&model_id);
    let price = crate::pricing::resolve_price(Some(&model_id));
    let mut lines = vec![
        Line::from(Span::styled(
            format!("  {model_id}"),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("  {}", price.label),
            Style::default().fg(DIM),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  cost     ", Style::default().fg(DIM)),
            Span::styled(
                format_usd(row.cost),
                Style::default().fg(heat(row.cost)).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  tokens   ", Style::default().fg(DIM)),
            Span::styled(format_tokens(row.input), Style::default().fg(COOL)),
            Span::styled(" in  ", Style::default().fg(DIM)),
            Span::styled(format_tokens(row.output), Style::default().fg(GOLD)),
            Span::styled(" out", Style::default().fg(DIM)),
        ]),
        Line::from(vec![
            Span::styled("  sessions ", Style::default().fg(DIM)),
            Span::styled(format!("{}", row.sessions), Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  rate     ", Style::default().fg(DIM)),
            Span::styled(
                format!("${} / ${} per 1M", price.input_per_m, price.output_per_m),
                Style::default().fg(MUTED),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("  sessions", Style::default().fg(DIM))),
    ];
    for s in sess.iter().take(20) {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:>8}  ", format_usd(s.usage.cost.total_cost)),
                Style::default().fg(heat(s.usage.cost.total_cost)),
            ),
            Span::styled(truncate(&s.title, 40), Style::default().fg(TEXT)),
        ]));
    }
    if sess.len() > 20 {
        lines.push(Line::from(Span::styled(
            format!("  +{} more", sess.len() - 20),
            Style::default().fg(DIM),
        )));
    }
    page(f, app, area, "model", lines);
}

fn draw_help(f: &mut Frame, app: &mut App, area: Rect) {
    let (header, body, footer) = zones(area);
    draw_header(f, app, header);
    draw_footer(f, app, footer);
    fill(f, body, BG);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body);

    fn kv(k: &str, d: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {k:<12}"), Style::default().fg(EMBER)),
            Span::styled(d.to_string(), Style::default().fg(MUTED)),
        ])
    }
    fn head(t: &str) -> Line<'static> {
        Line::from(Span::styled(
            format!("  {t}"),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ))
    }
    fn blank() -> Line<'static> {
        Line::from("")
    }

    let left = vec![
        head("NAV"),
        kv("1-6", "pages"),
        kv("tab", "next page"),
        kv("o", "overall"),
        kv("q", "quit"),
        blank(),
        head("LIST"),
        kv("j/k arrows", "move"),
        kv("enter", "detail"),
        kv("esc", "back"),
        kv("/ f", "search"),
        kv("r", "rescan"),
    ];

    let right = vec![
        head("FILTERS"),
        kv("t", "time range"),
        kv("s", "sort (↓ high first)"),
        kv("a", "toggle subagents"),
        blank(),
        head("MOUSE"),
        kv("click", "tabs / chips / rows"),
        kv("dbl-click", "open"),
        kv("wheel", "scroll"),
        kv("right-click", "back"),
    ];

    let border = || {
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(LINE))
            .style(Style::default().bg(BG))
    };

    f.render_widget(
        Paragraph::new(left)
            .style(Style::default().bg(BG))
            .block(border()),
        cols[0],
    );
    f.render_widget(Paragraph::new(right).style(Style::default().bg(BG)), cols[1]);

    app.hits.page_body = Some(RectHit::from_ratatui(body));
}
