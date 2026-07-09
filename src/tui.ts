import {
  type ScanResult,
  type SessionRecord,
  type SortKey,
  sortSessions,
  computeTotals,
} from "./scanner"
import {
  formatTokens,
  formatUsd,
  formatDate,
  formatDuration,
  pad,
  shortPath,
  truncate,
} from "./format"
import { OFFICIAL_PRICES, PRICING_SOURCE, PRICING_UPDATED } from "./pricing"

type View = "list" | "detail" | "pricing" | "help" | "models"

type State = {
  view: View
  sessions: SessionRecord[]
  allSessions: SessionRecord[]
  scan: ScanResult
  selected: number
  scroll: number
  sortKey: SortKey
  sortDesc: boolean
  query: string
  searching: boolean
  detailScroll: number
  detailMaxScroll: number
  cols: number
  rows: number
  /** 1-based terminal row of first list item (for mouse) */
  listTopRow: number
  listH: number
  lastClickAt: number
  lastClickIndex: number
}

// ── palette (bright truecolor - readable on dark terminals) ────────
const esc = (n: string) => `\x1b[${n}m`
const rgb = (r: number, g: number, b: number) => `\x1b[38;2;${r};${g};${b}m`
const bgRgb = (r: number, g: number, b: number) => `\x1b[48;2;${r};${g};${b}m`

const t = {
  reset: esc("0"),
  bold: esc("1"),
  // fg - lifted so it doesn't look muddy
  text: rgb(235, 236, 242),
  muted: rgb(160, 165, 180),
  faint: rgb(110, 115, 130),
  white: rgb(255, 255, 255),
  accent: rgb(120, 210, 255),
  accentDim: rgb(100, 175, 230),
  green: rgb(100, 240, 160),
  greenDim: rgb(70, 190, 120),
  yellow: rgb(255, 215, 90),
  orange: rgb(255, 170, 80),
  red: rgb(255, 120, 120),
  magenta: rgb(220, 160, 255),
  cyan: rgb(110, 235, 250),
  // bg surfaces
  bg: bgRgb(14, 14, 18),
  bgRaised: bgRgb(22, 24, 32),
  bgPanel: bgRgb(18, 20, 28),
  bgSel: bgRgb(40, 85, 130),
  bgHeader: bgRgb(18, 20, 28),
  bgFooter: bgRgb(18, 20, 28),
  bgPreview: bgRgb(18, 20, 28),
  border: rgb(70, 75, 90),
  selFg: rgb(255, 255, 255),
}

function stripAnsi(s: string): string {
  return s.replace(/\x1b\[[0-9;]*m/g, "")
}

function visLen(s: string): number {
  return stripAnsi(s).length
}

/** Paint a full terminal line: bg fill + content clipped to width.
 *  Any SGR reset mid-line is followed by bg again so gaps keep the surface color
 *  (otherwise "sort cost" etc. sit on default black while the left side has a panel bg). */
function paint(content: string, width: number, bg = t.bg): string {
  // \x1b[0m clears bg - re-apply surface after every reset
  const fixed = content.replace(/\x1b\[0m/g, `\x1b[0m${bg}`)
  const plain = stripAnsi(fixed)
  if (plain.length > width) {
    return bg + plain.slice(0, Math.max(0, width - 1)) + "…" + t.reset
  }
  return bg + fixed + " ".repeat(width - plain.length) + t.reset
}

function padVis(s: string, width: number, align: "left" | "right" = "left"): string {
  const plain = stripAnsi(s)
  if (plain.length > width) return plain.slice(0, Math.max(0, width - 1)) + "…"
  const gap = width - plain.length
  if (align === "right") return " ".repeat(gap) + s
  return s + " ".repeat(gap)
}

function repeat(ch: string, n: number): string {
  return ch.repeat(Math.max(0, n))
}

function hRule(width: number, bg = t.bg): string {
  return paint(`${t.border}${repeat("─", width)}${t.reset}`, width, bg)
}

// ── terminal lifecycle (alternate screen = real fullscreen app) ────

function enterApp() {
  process.stdout.write(
    "\x1b[?1049h" + // alternate screen
      "\x1b[?25l" + // hide cursor
      "\x1b[?1000h" + // mouse click
      "\x1b[?1002h" + // mouse drag
      "\x1b[?1003h" + // mouse move (optional; some terms need for wheel)
      "\x1b[?1006h" + // SGR mouse coords
      "\x1b[2J" +
      "\x1b[H" +
      "\x1b[?7l", // no autowrap
  )
}

function leaveApp() {
  process.stdout.write(
    "\x1b[?1000l" +
      "\x1b[?1002l" +
      "\x1b[?1003l" +
      "\x1b[?1006l" +
      "\x1b[?7h" +
      "\x1b[?25h" +
      "\x1b[?1049l" +
      t.reset,
  )
}

/** Parse SGR mouse: ESC [ < btn ; x ; y M/m  (1-based x/y) */
function parseMouse(seq: string): { btn: number; x: number; y: number; release: boolean } | null {
  const m = seq.match(/^\x1b\[<(\d+);(\d+);(\d+)([Mm])/)
  if (!m) return null
  return {
    btn: Number(m[1]),
    x: Number(m[2]),
    y: Number(m[3]),
    release: m[4] === "m",
  }
}

function moveHome() {
  // re-assert hidden cursor every frame (Windows Terminal can re-show it)
  process.stdout.write("\x1b[?25l\x1b[H")
}

function termSize(): { cols: number; rows: number } {
  return {
    cols: Math.max(60, process.stdout.columns || 100),
    rows: Math.max(16, process.stdout.rows || 30),
  }
}

function costColor(amount: number, priced: boolean): string {
  if (!priced) return t.muted
  if (amount >= 50) return t.red
  if (amount >= 10) return t.orange
  if (amount >= 1) return t.yellow
  if (amount > 0) return t.green
  return t.faint
}

function costLabel(s: SessionRecord): string {
  const n = s.usage.cost.totalCost
  if (s.usage.inputTokens === 0 && s.usage.outputTokens === 0) return "-"
  if (!s.usage.hasDetailedUsage) return `~${formatUsd(n)}`
  return formatUsd(n)
}

function modelShort(id: string): string {
  return id
    .replace("grok-composer-2.5-fast", "composer")
    .replace("grok-4.5", "4.5")
    .replace("grok-build", "build")
    .replace("grok-", "")
}

function sparkBar(ratio: number, width: number): string {
  if (width <= 0) return ""
  const r = Math.max(0, Math.min(1, ratio))
  const filled = Math.round(r * width)
  // lighter blocks - less garish than solid █ wall
  return "▓".repeat(filled) + "░".repeat(Math.max(0, width - filled))
}

// ── column layout ──────────────────────────────────────────────────

type Cols = {
  cost: number
  bar: number
  input: number
  output: number
  turns: number
  model: number
  date: number
  title: number
}

function colLayout(w: number): Cols {
  const cost = 10
  const bar = w >= 100 ? 7 : w >= 80 ? 5 : 0
  const input = 8
  const output = 8
  const turns = 4
  const model = 9
  const date = w >= 92 ? 11 : 0
  // " " before each col after first + leading space
  const parts = 1 + 1 + (bar ? 1 : 0) + 1 + 1 + 1 + 1 + (date ? 1 : 0) + 1
  const fixed = cost + bar + input + output + turns + model + date + parts
  const title = Math.max(14, w - fixed)
  return { cost, bar, input, output, turns, model, date, title }
}

// ── frames ─────────────────────────────────────────────────────────

function titleBar(state: State): string {
  const w = state.cols
  const bg = t.bgHeader
  const left =
    `${t.bold}${t.accent} ◆ gburn ${t.reset}${bg}` +
    `${t.muted}grok build · api cost${t.reset}${bg}`
  const right = `${t.faint}${state.scan.sessions.length} sessions${t.reset}${bg}`
  const gap = Math.max(1, w - visLen(left) - visLen(right))
  return paint(left + bg + " ".repeat(gap) + right, w, bg)
}

function statsBar(state: State): string {
  const w = state.cols
  const bg = t.bgRaised
  const totals = computeTotals(state.sessions)
  const parts = [
    `${t.faint}COST${t.reset}${bg} ${t.bold}${t.green}${formatUsd(totals.totalCost)}${t.reset}${bg}`,
    `${t.faint}IN${t.reset}${bg} ${t.cyan}${formatTokens(totals.inputTokens)}${t.reset}${bg}`,
    `${t.faint}OUT${t.reset}${bg} ${t.magenta}${formatTokens(totals.outputTokens)}${t.reset}${bg}`,
    `${t.faint}OK${t.reset}${bg} ${t.yellow}${totals.withDetailed}/${totals.sessions}${t.reset}${bg}`,
  ]
  if (state.query) {
    parts.push(
      `${t.faint}FIND${t.reset}${bg} ${t.orange}${truncate(state.query, 16)}${t.reset}${bg}`,
    )
  }
  const sep = `${t.faint}  │  ${t.reset}${bg}`
  return paint(" " + parts.join(sep), w, bg)
}

function tableHeader(cols: Cols, w: number): string {
  const cells =
    " " +
    padVis(`${t.faint}COST${t.reset}`, cols.cost, "right") +
    (cols.bar ? " " + padVis("", cols.bar) : "") +
    " " +
    padVis(`${t.faint}INPUT${t.reset}`, cols.input, "right") +
    " " +
    padVis(`${t.faint}OUTPUT${t.reset}`, cols.output, "right") +
    " " +
    padVis(`${t.faint}TRN${t.reset}`, cols.turns, "right") +
    " " +
    padVis(`${t.faint}MODEL${t.reset}`, cols.model) +
    (cols.date ? " " + padVis(`${t.faint}WHEN${t.reset}`, cols.date) : "") +
    " " +
    padVis(`${t.faint}TITLE${t.reset}`, cols.title)
  return paint(cells, w, t.bgPanel)
}

function renderRow(
  s: SessionRecord,
  selected: boolean,
  cols: Cols,
  maxCost: number,
  width: number,
): string {
  const bg = selected ? t.bgSel : t.bg
  const cc = selected ? t.selFg : costColor(s.usage.cost.totalCost, s.usage.cost.priced)
  const cIn = selected ? t.selFg : t.cyan
  const cOut = selected ? t.selFg : t.magenta
  const cMuted = selected ? rgb(170, 190, 210) : t.muted
  const cModel = selected ? t.selFg : t.accentDim
  const cFaint = selected ? rgb(150, 170, 195) : t.faint
  const cText = selected ? t.selFg : t.text
  const uncertain =
    !s.usage.hasDetailedUsage && (s.usage.inputTokens > 0 || s.usage.outputTokens > 0)

  const caret = selected ? `${t.accent}▌${t.reset}${bg}` : " "
  const costPart = padVis(`${cc}${costLabel(s)}${t.reset}${bg}`, cols.cost, "right")

  let barPart = ""
  if (cols.bar > 0) {
    const ratio = s.usage.cost.priced ? s.usage.cost.totalCost / maxCost : 0
    barPart =
      " " + padVis(`${cc}${sparkBar(ratio, cols.bar)}${t.reset}${bg}`, cols.bar)
  }

  const body =
    costPart +
    barPart +
    " " +
    padVis(`${cIn}${formatTokens(s.usage.inputTokens)}${t.reset}${bg}`, cols.input, "right") +
    " " +
    padVis(`${cOut}${formatTokens(s.usage.outputTokens)}${t.reset}${bg}`, cols.output, "right") +
    " " +
    padVis(
      `${cMuted}${String(s.turnCount || s.usage.streams)}${t.reset}${bg}`,
      cols.turns,
      "right",
    ) +
    " " +
    padVis(
      `${cModel}${modelShort(s.modelId)}${s.usage.byModel.length > 1 ? "+" : ""}${t.reset}${bg}`,
      cols.model,
    ) +
    (cols.date
      ? " " +
        padVis(
          `${cFaint}${(formatDate(s.lastActiveAt).length >= 16
            ? formatDate(s.lastActiveAt).slice(5, 16)
            : formatDate(s.lastActiveAt))}${t.reset}${bg}`,
          cols.date,
        )
      : "") +
    " " +
    padVis(
      `${s.isSubagent ? `${cFaint}↳ ${t.reset}${bg}` : ""}` +
        `${s.title.startsWith("019f") ? cFaint : cText}${s.title}` +
        `${uncertain ? "?" : ""}` +
        `${s.childSessionIds.length ? `${cFaint} [${s.childSessionIds.length} sub]${t.reset}${bg}` : ""}${t.reset}${bg}`,
      cols.title,
    )

  return paint(caret + body, width, bg)
}

// ── preview panel (fills dead space under list) ────────────────────

function previewPanel(state: State, height: number): string[] {
  const w = state.cols
  const lines: string[] = []
  if (height <= 0) return lines

  lines.push(hRule(w, t.bgPreview))
  if (height === 1) return lines

  const s = state.sessions[state.selected]
  if (!s) {
    lines.push(paint(` ${t.faint}no selection${t.reset}`, w, t.bgPreview))
    while (lines.length < height) lines.push(paint("", w, t.bgPreview))
    return lines
  }

  const u = s.usage
  const cost = costLabel(s)
  const cc = costColor(u.cost.totalCost, u.cost.priced)

  const tag = s.isSubagent
    ? `${t.orange}sub${t.reset}${t.bgPreview} `
    : s.childSessionIds.length
      ? `${t.accentDim}${s.childSessionIds.length} subagents${t.reset}${t.bgPreview} `
      : ""

  // line 1: title + cost
  lines.push(
    paint(
      ` ${t.faint}▸${t.reset}${t.bgPreview} ${tag}${t.bold}${t.white}${truncate(s.title, Math.max(10, w - 28))}${t.reset}` +
        `${t.bgPreview}  ${cc}${t.bold}${cost}${t.reset}`,
      w,
      t.bgPreview,
    ),
  )

  if (lines.length >= height) return lines.slice(0, height)

  // line 2: meta
  const modelBit =
    s.usage.byModel.length > 1
      ? s.usage.byModel.map((m) => modelShort(m.modelId)).join("+")
      : modelShort(s.modelId)
  lines.push(
    paint(
      `   ${t.faint}${modelBit}${t.reset}${t.bgPreview}` +
        `${t.faint} · ${formatDate(s.lastActiveAt)} · ${s.turnCount} turns · ${s.toolCallCount} tools${t.reset}` +
        `${t.bgPreview}${t.faint} · ${shortPath(s.cwd, Math.max(12, w - 55))}${t.reset}`,
      w,
      t.bgPreview,
    ),
  )

  if (lines.length >= height) return lines.slice(0, height)

  // line 3: tokens + rates
  lines.push(
    paint(
      `   ${t.cyan}in ${formatTokens(u.inputTokens)}${t.reset}${t.bgPreview}` +
        `  ${t.magenta}out ${formatTokens(u.outputTokens)}${t.reset}${t.bgPreview}` +
        `  ${t.faint}streams ${u.streams}${t.reset}${t.bgPreview}` +
        `  ${u.hasDetailedUsage ? t.greenDim + "detailed" : t.orange + "approx"}${t.reset}` +
        `${t.bgPreview}  ${t.faint}@ $${u.cost.price.inputPerM}/$${u.cost.price.outputPerM} per 1M${t.reset}`,
      w,
      t.bgPreview,
    ),
  )

  while (lines.length < height) {
    lines.push(paint("", w, t.bgPreview))
  }
  return lines.slice(0, height)
}

// ── footer ─────────────────────────────────────────────────────────

function footerBar(state: State): string {
  const w = state.cols
  const bg = t.bgFooter
  let keys: string
  if (state.searching) {
    keys =
      `${t.yellow}search ${t.reset}${bg}${t.white}${state.query}_${t.reset}${bg}` +
      `${t.faint}  enter · esc${t.reset}${bg}`
  } else if (state.view === "list") {
    keys = hints(
      [
        ["↑↓", "move"],
        ["click", "select"],
        ["dbl", "open"],
        ["/", "find"],
        ["s", "sort"],
        ["p", "price"],
        ["m", "models"],
        ["q", "quit"],
      ],
      bg,
    )
  } else {
    keys = hints(
      [
        ["esc", "back"],
        ["↑↓", "scroll"],
        ["q", "quit"],
      ],
      bg,
    )
  }

  const right =
    state.view === "list"
      ? `${t.faint}sort ${t.muted}${state.sortKey}${state.sortDesc ? " ↓" : " ↑"}${t.reset}${bg}`
      : `${t.faint}${state.view}${t.reset}${bg}`

  const left = " " + keys
  const gap = Math.max(1, w - visLen(left) - visLen(right))
  return paint(left + bg + " ".repeat(gap) + right, w, bg)
}

function hints(items: [string, string][], bg: string): string {
  return items
    .map(
      ([k, label]) =>
        `${t.accent}${k}${t.reset}${bg}${t.faint} ${label}${t.reset}${bg}`,
    )
    .join(`${t.faint}  ${t.reset}${bg}`)
}

// ── main list view ─────────────────────────────────────────────────

function renderListView(state: State): string[] {
  const w = state.cols
  const h = state.rows
  const lines: string[] = []

  const titleH = 1
  const statsH = 1
  const ruleH = 1
  const theadH = 1
  const theadRuleH = 1
  const footerH = 1
  const previewH = h >= 22 ? 4 : h >= 18 ? 3 : 0
  const chrome =
    titleH + statsH + ruleH + theadH + theadRuleH + footerH + previewH
  const listH = Math.max(3, h - chrome)
  // rows above first data line: title, stats, rule, thead, theadRule = 5 (1-based row 6)
  state.listTopRow = titleH + statsH + ruleH + theadH + theadRuleH + 1
  state.listH = listH

  const cols = colLayout(w)
  const maxCost = Math.max(
    0.01,
    ...state.sessions.map((s) => (s.usage.cost.priced ? s.usage.cost.totalCost : 0)),
  )

  lines.push(titleBar(state))
  lines.push(statsBar(state))
  lines.push(hRule(w, t.bg))
  lines.push(tableHeader(cols, w))
  lines.push(hRule(w, t.bgPanel))

  // scroll window
  if (state.sessions.length === 0) {
    lines.push(paint(` ${t.muted}No sessions.${t.reset}`, w, t.bg))
    while (lines.length < titleH + statsH + ruleH + theadH + theadRuleH + listH) {
      lines.push(paint("", w, t.bg))
    }
  } else {
    if (state.selected < state.scroll) state.scroll = state.selected
    if (state.selected >= state.scroll + listH) {
      state.scroll = state.selected - listH + 1
    }

    const end = Math.min(state.sessions.length, state.scroll + listH)
    for (let i = state.scroll; i < end; i++) {
      lines.push(renderRow(state.sessions[i], i === state.selected, cols, maxCost, w))
    }
    // pad remaining list rows
    const drawn = end - state.scroll
    for (let i = drawn; i < listH; i++) {
      lines.push(paint("", w, t.bg))
    }
  }

  // preview uses the dead space
  if (previewH > 0) {
    lines.push(...previewPanel(state, previewH))
  }

  lines.push(footerBar(state))

  while (lines.length < h) lines.push(paint("", w, t.bg))
  return lines.slice(0, h)
}

// ── other views ────────────────────────────────────────────────────

function renderPaged(state: State, body: string[]): string[] {
  const w = state.cols
  const h = state.rows
  const lines: string[] = []
  lines.push(titleBar(state))
  lines.push(hRule(w, t.bg))

  const footer = footerBar(state)
  const budget = Math.max(1, h - 3) // title + rule + footer
  const maxScroll = Math.max(0, body.length - budget)
  state.detailMaxScroll = maxScroll
  state.detailScroll = Math.min(Math.max(0, state.detailScroll), maxScroll)
  const slice = body.slice(state.detailScroll, state.detailScroll + budget)
  for (const row of slice) lines.push(paint(row, w, t.bg))
  while (lines.length < h - 1) lines.push(paint("", w, t.bg))
  lines.push(footer)
  while (lines.length < h) lines.push(paint("", w, t.bg))
  return lines.slice(0, h)
}

function renderDetail(state: State): string[] {
  const s = state.sessions[state.selected]
  if (!s) return renderPaged(state, [` ${t.muted}No session selected.${t.reset}`])

  const w = Math.max(40, state.cols - 2)
  const u = s.usage
  const body: string[] = []

  body.push(` ${t.bold}${t.white}${truncate(s.title, w)}${t.reset}`)
  body.push(` ${t.faint}${s.id}${t.reset}`)
  body.push(` ${t.border}${repeat("─", Math.min(w, 48))}${t.reset}`)

  const kv = (k: string, v: string) =>
    ` ${t.faint}${pad(k, 14)}${t.reset} ${t.text}${v}${t.reset}`

  for (const [k, v] of [
    ["Project", shortPath(s.cwd, w - 16)],
    ["Model", s.modelId],
    ["Models used", s.modelsUsed.join(", ") || "-"],
    ["Agent", s.agentName || "-"],
    ["Reasoning", s.reasoningEffort || "-"],
    ["Created", formatDate(s.createdAt)],
    ["Last active", formatDate(s.lastActiveAt)],
    ["Duration", formatDuration(s.sessionDurationSeconds)],
    ["Turns", String(s.turnCount)],
    ["Tool calls", String(s.toolCallCount)],
    ["Messages", String(s.numMessages)],
    [
      "Context peak",
      `${formatTokens(s.contextTokensUsed)} / ${formatTokens(s.contextWindowTokens)}`,
    ],
    ["Streams", String(u.streams)],
    ["Source", u.hasDetailedUsage ? "updates.jsonl streams" : "signals fallback"],
    [
      "Subagent",
      s.isSubagent
        ? `yes · ${s.subagentType || "?"} · parent ${s.parentSessionId?.slice(0, 8) || "-"}`
        : s.childSessionIds.length
          ? `parent of ${s.childSessionIds.length} child session(s)`
          : "no",
    ],
  ] as [string, string][]) {
    body.push(kv(k, v))
  }
  if (s.isSubagent && s.subagentDescription) {
    body.push(kv("Sub task", truncate(s.subagentDescription, w - 16)))
  }

  body.push(` ${t.border}${repeat("─", Math.min(w, 48))}${t.reset}`)
  body.push(` ${t.bold}${t.accent}Tokens${t.reset}`)
  body.push(
    ` ${t.faint}input ${t.reset}${t.cyan}${formatTokens(u.inputTokens)}${t.reset}` +
      `${t.faint}  (${u.inputTokens.toLocaleString()})${t.reset}` +
      `   ${t.faint}output ${t.reset}${t.magenta}${formatTokens(u.outputTokens)}${t.reset}` +
      `${t.faint}  (${u.outputTokens.toLocaleString()})${t.reset}`,
  )

  body.push(` ${t.border}${repeat("─", Math.min(w, 48))}${t.reset}`)
  body.push(` ${t.bold}${t.green}API cost${t.reset}`)
  body.push(
    ` ${t.faint}input  ${t.reset}${formatUsd(u.cost.inputCost)}` +
      `   ${t.faint}output ${t.reset}${formatUsd(u.cost.outputCost)}` +
      `   ${t.bold}${t.green}total ${formatUsd(u.cost.totalCost)}${t.reset}`,
  )

  if (u.byModel.length > 1) {
    body.push(` ${t.bold}${t.white}By model${t.reset}`)
    for (const m of u.byModel) {
      const pct = Math.round(m.weight * 100)
      body.push(
        ` ${t.accentDim}${pad(truncate(modelShort(m.modelId), 12), 12)}${t.reset}` +
          ` ${t.cyan}${padVis(formatTokens(m.inputTokens), 8, "right")}${t.reset} in` +
          ` ${t.magenta}${padVis(formatTokens(m.outputTokens), 8, "right")}${t.reset} out` +
          `  ${costColor(m.cost.totalCost, m.cost.priced)}${formatUsd(m.cost.totalCost)}${t.reset}` +
          `  ${t.faint}${pct}% · $${m.cost.price.inputPerM}/$${m.cost.price.outputPerM}${t.reset}`,
      )
    }
  }

  const sum = u.cost.inputCost + u.cost.outputCost
  if (sum > 0) {
    const barW = Math.min(36, w - 8)
    const inShare = u.cost.inputCost / sum
    const inBars = Math.round(inShare * barW)
    body.push(
      ` ${t.cyan}${repeat("▓", inBars)}${t.magenta}${repeat("▓", barW - inBars)}${t.reset}` +
        `  ${t.faint}in ${Math.round(inShare * 100)}% · out ${Math.round((1 - inShare) * 100)}%${t.reset}`,
    )
  }

  if (u.streamsDetail.length > 0) {
    body.push(` ${t.border}${repeat("─", Math.min(w, 48))}${t.reset}`)
    body.push(` ${t.bold}${t.white}Top streams${t.reset}`)
    const top = [...u.streamsDetail]
      .sort((a, b) => b.inputTokens - a.inputTokens)
      .slice(0, 10)
    const maxIn = Math.max(1, ...top.map((x) => x.inputTokens))
    for (const st of top) {
      body.push(
        ` ${t.cyan}${padVis(formatTokens(st.inputTokens), 7, "right")}${t.reset}` +
          ` ${t.faint}in${t.reset}` +
          `  ${t.magenta}${padVis(formatTokens(st.outputTokens), 7, "right")}${t.reset}` +
          ` ${t.faint}out${t.reset}` +
          `  ${t.greenDim}${sparkBar(st.inputTokens / maxIn, 12)}${t.reset}`,
      )
    }
  }

  return renderPaged(state, body)
}

function renderPricing(state: State): string[] {
  const body: string[] = []
  body.push(
    ` ${t.bold}${t.accent}Grok Build models · list prices${t.reset}  ${t.faint}${PRICING_UPDATED}${t.reset}`,
  )
  body.push(` ${t.faint}xAI: ${PRICING_SOURCE}${t.reset}`)
  body.push(` ${t.faint}Composer: Cursor model pricing${t.reset}`)
  body.push(
    ` ${t.muted}Only models that appear in Grok Build (region-dependent)${t.reset}`,
  )
  body.push("")
  body.push(
    " " +
      padVis(`${t.faint}MODEL${t.reset}`, 28) +
      padVis(`${t.faint}INPUT${t.reset}`, 10, "right") +
      padVis(`${t.faint}CACHED${t.reset}`, 10, "right") +
      padVis(`${t.faint}OUTPUT${t.reset}`, 10, "right") +
      padVis(`${t.faint}CTX${t.reset}`, 8, "right"),
  )
  body.push(` ${t.border}${repeat("─", 68)}${t.reset}`)

  for (const p of OFFICIAL_PRICES) {
    if (p.id === "grok-build-0.1") continue

    const inStr = p.inputPerM > 0 ? `$${p.inputPerM.toFixed(2)}` : "-"
    const outStr = p.outputPerM > 0 ? `$${p.outputPerM.toFixed(2)}` : "-"
    const cached =
      p.cachedInputPerM != null ? `$${p.cachedInputPerM.toFixed(2)}` : "-"

    body.push(
      " " +
        padVis(`${t.text}${p.label}${t.reset}`, 28) +
        padVis(`${t.cyan}${inStr}${t.reset}`, 10, "right") +
        padVis(`${t.muted}${cached}${t.reset}`, 10, "right") +
        padVis(`${t.magenta}${outStr}${t.reset}`, 10, "right") +
        padVis(`${t.faint}${p.context || "-"}${t.reset}`, 8, "right"),
    )
    if (p.note) body.push(`   ${t.faint}${p.note}${t.reset}`)
  }
  body.push("")
  return renderPaged(state, body)
}

function renderModels(state: State): string[] {
  const totals = computeTotals(state.allSessions)
  const body: string[] = []
  body.push(` ${t.bold}${t.accent}Usage by model${t.reset}`)
  body.push("")
  body.push(
    " " +
      padVis(`${t.faint}MODEL${t.reset}`, 26) +
      padVis(`${t.faint}SESS${t.reset}`, 8, "right") +
      padVis(`${t.faint}INPUT${t.reset}`, 12, "right") +
      padVis(`${t.faint}OUTPUT${t.reset}`, 12, "right") +
      padVis(`${t.faint}COST${t.reset}`, 12, "right"),
  )
  body.push(` ${t.border}${repeat("─", 70)}${t.reset}`)

  const rows = [...totals.byModel.entries()].sort((a, b) => b[1].cost - a[1].cost)
  const maxCost = Math.max(0.01, ...rows.map(([, r]) => r.cost))

  for (const [model, row] of rows) {
    const costStr = row.cost > 0 || row.input > 0 || row.output > 0 ? formatUsd(row.cost) : "-"
    body.push(
      " " +
        padVis(`${t.text}${truncate(model, 26)}${t.reset}`, 26) +
        padVis(`${t.muted}${row.sessions}${t.reset}`, 8, "right") +
        padVis(`${t.cyan}${formatTokens(row.input)}${t.reset}`, 12, "right") +
        padVis(`${t.magenta}${formatTokens(row.output)}${t.reset}`, 12, "right") +
        padVis(
          `${costColor(row.cost, true)}${costStr}${t.reset}`,
          12,
          "right",
        ) +
        ` ${t.greenDim}${sparkBar(row.cost / maxCost, 8)}${t.reset}`,
    )
  }
  body.push(` ${t.border}${repeat("─", 70)}${t.reset}`)
  body.push(
    " " +
      padVis(`${t.bold}total${t.reset}`, 26) +
      padVis(String(totals.sessions), 8, "right") +
      padVis(formatTokens(totals.inputTokens), 12, "right") +
      padVis(formatTokens(totals.outputTokens), 12, "right") +
      padVis(`${t.green}${formatUsd(totals.totalCost)}${t.reset}`, 12, "right"),
  )
  return renderPaged(state, body)
}

function renderHelp(state: State): string[] {
  const row = (k: string, d: string) =>
    `  ${t.accent}${pad(k, 12)}${t.reset}${t.muted}${d}${t.reset}`
  return renderPaged(state, [
    ` ${t.bold}${t.accent}Help${t.reset}`,
    "",
    ` ${t.bold}Display${t.reset}`,
    ` ${t.muted}Fullscreen alternate-screen TUI. Scrollback restored on quit.${t.reset}`,
    ` ${t.muted}Run with:  npx @wiktorekdev/gburn${t.reset}`,
    "",
    ` ${t.bold}Keys${t.reset}`,
    row("↑ ↓ / j k", "move"),
    row("click", "select row"),
    row("dbl-click", "open detail"),
    row("wheel", "scroll"),
    row("Enter", "open detail"),
    row("/  f", "search"),
    row("s", "sort"),
    row("p", "prices"),
    row("m", "by model"),
    row("r", "rescan"),
    row("q", "quit"),
    "",
  ])
}

// ── state helpers ──────────────────────────────────────────────────

function applyFilter(state: State) {
  const q = state.query.trim().toLowerCase()
  let list = state.allSessions
  if (q) {
    list = list.filter(
      (s) =>
        s.title.toLowerCase().includes(q) ||
        s.cwd.toLowerCase().includes(q) ||
        s.modelId.toLowerCase().includes(q) ||
        s.id.toLowerCase().includes(q) ||
        s.modelsUsed.some((m) => m.toLowerCase().includes(q)),
    )
  }
  state.sessions = sortSessions(list, state.sortKey, state.sortDesc)
  state.selected = Math.min(state.selected, Math.max(0, state.sessions.length - 1))
  if (state.selected < 0) state.selected = 0
  state.scroll = 0
}

function cycleSort(state: State) {
  const order: SortKey[] = ["cost", "date", "input", "output", "turns", "title"]
  const idx = order.indexOf(state.sortKey)
  if (idx === order.length - 1) {
    if (state.sortDesc) {
      state.sortDesc = false
      state.sortKey = "cost"
    } else {
      state.sortDesc = true
      state.sortKey = "cost"
    }
  } else {
    state.sortKey = order[idx + 1]
    state.sortDesc = true
  }
  applyFilter(state)
}

function frame(state: State): string[] {
  const size = termSize()
  state.cols = size.cols
  state.rows = size.rows

  if (state.view === "list") return renderListView(state)
  if (state.view === "detail") return renderDetail(state)
  if (state.view === "pricing") return renderPricing(state)
  if (state.view === "models") return renderModels(state)
  return renderHelp(state)
}

function draw(state: State) {
  const lines = frame(state)
  // exact grid write into alt screen - no scrollback pollution
  moveHome()
  process.stdout.write(lines.join("\n"))
}

export async function runTui(
  initial: ScanResult,
  rescan: () => Promise<ScanResult>,
): Promise<void> {
  const state: State = {
    view: "list",
    sessions: [],
    allSessions: initial.sessions,
    scan: initial,
    selected: 0,
    scroll: 0,
    sortKey: "cost",
    sortDesc: true,
    query: "",
    searching: false,
    detailScroll: 0,
    detailMaxScroll: 0,
    listTopRow: 6,
    listH: 10,
    lastClickAt: 0,
    lastClickIndex: -1,
    ...termSize(),
  }
  applyFilter(state)

  if (!process.stdin.isTTY || !process.stdout.isTTY) {
    printSummary(initial)
    return
  }

  enterApp()
  process.stdin.setRawMode(true)
  process.stdin.resume()
  process.stdin.setEncoding("utf8")

  let closed = false
  const cleanup = () => {
    if (closed) return
    closed = true
    try {
      process.stdin.setRawMode(false)
    } catch {
      /* ignore */
    }
    leaveApp()
  }

  process.on("exit", cleanup)
  process.on("SIGINT", () => {
    cleanup()
    process.exit(0)
  })

  const onResize = () => draw(state)
  process.stdout.on("resize", onResize)

  draw(state)

  await new Promise<void>((resolve) => {
    let buf = ""

    const handleMouse = (mouse: {
      btn: number
      x: number
      y: number
      release: boolean
    }) => {
      if (mouse.release) return

      // wheel: 64 up, 65 down (SGR)
      if (mouse.btn === 64) {
        if (state.view === "list") {
          state.selected = Math.max(0, state.selected - 1)
        } else {
          state.detailScroll = Math.max(0, state.detailScroll - 1)
        }
        draw(state)
        return
      }
      if (mouse.btn === 65) {
        if (state.view === "list") {
          state.selected = Math.min(
            Math.max(0, state.sessions.length - 1),
            state.selected + 1,
          )
        } else {
          state.detailScroll = Math.min(
            state.detailMaxScroll,
            state.detailScroll + 1,
          )
        }
        draw(state)
        return
      }

      // left click
      if (mouse.btn === 0 && state.view === "list" && !state.searching) {
        const rowInList = mouse.y - state.listTopRow
        if (rowInList >= 0 && rowInList < state.listH) {
          const idx = state.scroll + rowInList
          if (idx >= 0 && idx < state.sessions.length) {
            const now = Date.now()
            const dbl =
              idx === state.lastClickIndex && now - state.lastClickAt < 400
            state.selected = idx
            state.lastClickAt = now
            state.lastClickIndex = idx
            if (dbl) {
              state.view = "detail"
              state.detailScroll = 0
            }
            draw(state)
          }
        }
        return
      }

      // click outside list in detail = ignore; optional: future footer buttons
    }

    const onData = async (chunk: string) => {
      if (closed) return
      buf += chunk

      // drain complete mouse sequences and key events from buffer
      while (buf.length > 0) {
        // SGR mouse
        if (buf.startsWith("\x1b[<")) {
          const end = buf.search(/[Mm]/)
          if (end === -1) return // incomplete
          const seq = buf.slice(0, end + 1)
          buf = buf.slice(end + 1)
          const mouse = parseMouse(seq)
          if (mouse) handleMouse(mouse)
          continue
        }

        // CSI sequences (arrows, pages)
        if (buf.startsWith("\x1b[")) {
          // need full CSI: ESC [ ... letter
          const m = buf.match(/^\x1b\[[0-9;]*[A-Za-z~]/)
          if (!m) {
            if (buf.length > 16) buf = buf.slice(1)
            return
          }
          const key = m[0]
          buf = buf.slice(key.length)
          await handleKey(key)
          continue
        }

        if (buf.startsWith("\x1b")) {
          // bare esc or incomplete
          if (buf.length === 1) {
            // wait a tick for multi-byte; if alone, treat as esc
            setTimeout(() => {
              if (buf === "\x1b") {
                buf = ""
                void handleKey("\x1b")
              }
            }, 20)
            return
          }
          const key = buf[0]
          buf = buf.slice(1)
          await handleKey(key)
          continue
        }

        const key = buf[0]
        buf = buf.slice(1)
        await handleKey(key)
      }
    }

    const handleKey = async (key: string) => {
      if (closed) return

      if (key === "\u0003") {
        cleanup()
        resolve()
        return
      }

      if (state.searching) {
        if (key === "\x1b") {
          state.searching = false
          state.query = ""
          applyFilter(state)
        } else if (key === "\r" || key === "\n") {
          state.searching = false
          applyFilter(state)
        } else if (key === "\x7f" || key === "\b") {
          state.query = state.query.slice(0, -1)
          applyFilter(state)
        } else if (key.length === 1 && key >= " ") {
          state.query += key
          applyFilter(state)
        }
        draw(state)
        return
      }

      if (key === "\x1b[A" || key === "k") {
        if (state.view === "list") {
          state.selected = Math.max(0, state.selected - 1)
        } else {
          state.detailScroll = Math.max(0, state.detailScroll - 1)
        }
      } else if (key === "\x1b[B" || key === "j") {
        if (state.view === "list") {
          state.selected = Math.min(
            Math.max(0, state.sessions.length - 1),
            state.selected + 1,
          )
        } else {
          state.detailScroll = Math.min(
            state.detailMaxScroll,
            state.detailScroll + 1,
          )
        }
      } else if (key === "\x1b[5~") {
        if (state.view === "list") {
          state.selected = Math.max(0, state.selected - 10)
        } else {
          state.detailScroll = Math.max(0, state.detailScroll - 10)
        }
      } else if (key === "\x1b[6~") {
        if (state.view === "list") {
          state.selected = Math.min(
            Math.max(0, state.sessions.length - 1),
            state.selected + 10,
          )
        } else {
          state.detailScroll = Math.min(
            state.detailMaxScroll,
            state.detailScroll + 10,
          )
        }
      } else if (key === "\x1b" || key === "\x1b\x1b") {
        if (state.view !== "list") {
          state.view = "list"
          state.detailScroll = 0
        }
      } else if (key === "\r" || key === "\n") {
        if (state.view === "list" && state.sessions[state.selected]) {
          state.view = "detail"
          state.detailScroll = 0
        }
      } else if (key === "\x7f" || key === "\b") {
        if (state.view !== "list") {
          state.view = "list"
          state.detailScroll = 0
        }
      } else if (key === "/" || key === "f") {
        state.view = "list"
        state.searching = true
      } else if (key === "s") {
        cycleSort(state)
      } else if (key === "p") {
        state.view = "pricing"
        state.detailScroll = 0
      } else if (key === "m") {
        state.view = "models"
        state.detailScroll = 0
      } else if (key === "?" || key === "h") {
        state.view = "help"
        state.detailScroll = 0
      } else if (key === "r") {
        const next = await rescan()
        state.scan = next
        state.allSessions = next.sessions
        applyFilter(state)
      } else if (key === "g") {
        state.selected = 0
      } else if (key === "G") {
        state.selected = Math.max(0, state.sessions.length - 1)
      } else if (key === "q") {
        cleanup()
        resolve()
        return
      }

      draw(state)
    }

    process.stdin.on("data", onData)
  })

  process.stdout.off("resize", onResize)
}

export function printSummary(scan: ScanResult) {
  const totals = scan.totals
  console.log(`${t.bold}${t.accent}◆ gburn${t.reset}  ${t.muted}grok build · api cost${t.reset}`)
  console.log(`${t.faint}${scan.sessionsDir}${t.reset}`)
  console.log()
  console.log(
    `${t.faint}sessions${t.reset} ${totals.sessions}` +
      `  ${t.faint}cost${t.reset} ${t.green}${formatUsd(totals.totalCost)}${t.reset}`,
  )
  console.log(
    `${t.faint}tokens${t.reset}  ${t.cyan}in ${formatTokens(totals.inputTokens)}${t.reset}` +
      `  ${t.magenta}out ${formatTokens(totals.outputTokens)}${t.reset}` +
      `  ${t.muted}sum ${formatTokens(totals.totalTokens)}${t.reset}`,
  )
  console.log()

  const rows = sortSessions(scan.sessions, "cost", true).slice(0, 25)
  console.log(
    pad("COST", 10, "right") +
      pad("INPUT", 10, "right") +
      pad("OUTPUT", 10, "right") +
      "  " +
      pad("MODEL", 12) +
      "TITLE",
  )
  console.log(repeat("-", 80))
  for (const s of rows) {
    const cost = costLabel(s)
    const cc = costColor(s.usage.cost.totalCost, true)
    console.log(
      `${cc}${pad(cost, 10, "right")}${t.reset}` +
        pad(formatTokens(s.usage.inputTokens), 10, "right") +
        pad(formatTokens(s.usage.outputTokens), 10, "right") +
        "  " +
        pad(truncate(modelShort(s.modelId), 12), 12) +
        truncate(s.title, 40) +
        (s.usage.hasDetailedUsage ? "" : " ?"),
    )
  }
  if (scan.sessions.length > 25) {
    console.log(`${t.faint}… +${scan.sessions.length - 25} more${t.reset}`)
  }
  console.log()
  console.log(`${t.faint}npx @wiktorekdev/gburn  · ${PRICING_SOURCE}${t.reset}`)
}

export function printJson(scan: ScanResult) {
  const ordered = sortSessions(scan.sessions, "cost", true)
  const payload = {
    scannedAt: scan.scannedAt,
    grokHome: scan.grokHome,
    sessionsDir: scan.sessionsDir,
    pricingSource: PRICING_SOURCE,
    totals: {
      sessions: scan.totals.sessions,
      withDetailed: scan.totals.withDetailed,
      inputTokens: scan.totals.inputTokens,
      outputTokens: scan.totals.outputTokens,
      totalTokens: scan.totals.totalTokens,
      totalCost: scan.totals.totalCost,
      byModel: Object.fromEntries(scan.totals.byModel),
    },
    sessions: ordered.map((s) => ({
      id: s.id,
      title: s.title,
      cwd: s.cwd,
      modelId: s.modelId,
      modelsUsed: s.modelsUsed,
      createdAt: s.createdAt,
      lastActiveAt: s.lastActiveAt,
      turnCount: s.turnCount,
      toolCallCount: s.toolCallCount,
      contextTokensUsed: s.contextTokensUsed,
      inputTokens: s.usage.inputTokens,
      outputTokens: s.usage.outputTokens,
      streams: s.usage.streams,
      hasDetailedUsage: s.usage.hasDetailedUsage,
      cost: s.usage.cost.totalCost,
      inputCost: s.usage.cost.inputCost,
      outputCost: s.usage.cost.outputCost,
      priceModel: s.usage.cost.price.id,
      byModel: s.usage.byModel.map((m) => ({
        modelId: m.modelId,
        inputTokens: m.inputTokens,
        outputTokens: m.outputTokens,
        weight: m.weight,
        cost: m.cost.totalCost,
      })),
      isSubagent: s.isSubagent,
      parentSessionId: s.parentSessionId,
      childSessionIds: s.childSessionIds,
      subagentType: s.subagentType,
    })),
  }
  console.log(JSON.stringify(payload, null, 2))
}
