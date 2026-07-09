/** Write docs/screenshot.svg — anonymized demo TUI frame for README. */
import { writeFileSync, mkdirSync } from "fs"
import { join, dirname } from "path"
import { fileURLToPath } from "url"

const __dir = dirname(fileURLToPath(import.meta.url))
const outDir = join(__dir, "..", "docs")
mkdirSync(outDir, { recursive: true })

const W = 1100
const H = 640
const PAD = 20
const ROW_H = 22
const FONT = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"

const rows = [
  { cost: "$147.69", bar: 1.0, inn: "72.7M", out: "379k", trn: "78", model: "4.5", when: "07-09 17:52", title: "Landing page redesign · conversion layout", sel: true },
  { cost: "$32.49", bar: 0.22, inn: "15.8M", out: "161k", trn: "18", model: "4.5", when: "07-09 21:34", title: "Internal metrics dashboard + charts", sel: false },
  { cost: "$24.47", bar: 0.17, inn: "11.8M", out: "140k", trn: "12", model: "4.5", when: "07-09 20:55", title: "Game web port · runtime fidelity [2 sub]", sel: false },
  { cost: "$13.03", bar: 0.09, inn: "6.33M", out: "59.7k", trn: "7", model: "4.5", when: "07-09 18:51", title: "PR review pass on auth middleware", sel: false },
  { cost: "$5.08", bar: 0.03, inn: "2.31M", out: "74.7k", trn: "1", model: "4.5", when: "07-09 20:41", title: "↳ Asset extraction pass", sel: false },
  { cost: "$2.70", bar: 0.02, inn: "1.17M", out: "61.2k", trn: "1", model: "4.5", when: "07-09 20:39", title: "↳ Runtime polish + input mapping", sel: false },
  { cost: "$1.07", bar: 0.01, inn: "490k", out: "15.3k", trn: "5", model: "4.5", when: "07-09 13:48", title: "Scaffold CLI for log parsing", sel: false },
  { cost: "$0.11", bar: 0.005, inn: "26.0k", out: "10.1k", trn: "1", model: "4.5", when: "07-09 13:41", title: "Brainstorm side-project ideas", sel: false },
  { cost: "$0.07", bar: 0.004, inn: "49.6k", out: "11.8k", trn: "3", model: "build", when: "07-09 14:36", title: "Debug DNS on home lab DNS box", sel: false },
  { cost: "$0.04", bar: 0.002, inn: "10.0k", out: "2.4k", trn: "1", model: "composer", when: "07-09 17:53", title: "Quick edit on README badges", sel: false },
]

function esc(s) {
  return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
}

function costColor(c) {
  const n = parseFloat(String(c).replace(/[$,]/g, ""))
  if (n >= 50) return "#ff7878"
  if (n >= 10) return "#ffd75a"
  if (n >= 1) return "#ffd75a"
  if (n > 0) return "#64f0a0"
  return "#9aa0b0"
}

function barSvg(ratio, x, y, w = 56, h = 10) {
  const filled = Math.round(Math.max(0, Math.min(1, ratio)) * w)
  return (
    `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="#2a2e3a" rx="2"/>` +
    `<rect x="${x}" y="${y}" width="${filled}" height="${h}" fill="#64f0a0" opacity="0.85" rx="2"/>`
  )
}

const y0 = 108
const body = rows
  .map((r, i) => {
    const yy = y0 + i * ROW_H
    const fg = r.sel ? "#ffffff" : "#ebebf2"
    const cc = r.sel ? "#ffffff" : costColor(r.cost)
    const cyan = r.sel ? "#ffffff" : "#6eebfa"
    const mag = r.sel ? "#ffffff" : "#dca0ff"
    const muted = r.sel ? "#ffffff" : "#9aa0b0"
    const blue = r.sel ? "#ffffff" : "#64afe6"
    const bg = r.sel
      ? `<rect x="${PAD}" y="${yy - 15}" width="${W - PAD * 2}" height="${ROW_H}" fill="#285582" rx="3"/>`
      : ""
    const caret = r.sel ? "▌" : " "
    return [
      bg,
      `<text x="${PAD + 8}" y="${yy}" fill="${fg}" font-family="${FONT}" font-size="13">${caret}</text>`,
      `<text x="${PAD + 22}" y="${yy}" fill="${cc}" font-family="${FONT}" font-size="13" font-weight="600">${esc(r.cost.padStart(8))}</text>`,
      barSvg(r.bar, PAD + 100, yy - 9),
      `<text x="${PAD + 210}" y="${yy}" fill="${cyan}" font-family="${FONT}" font-size="13" text-anchor="end">${esc(r.inn.padStart(7))}</text>`,
      `<text x="${PAD + 280}" y="${yy}" fill="${mag}" font-family="${FONT}" font-size="13" text-anchor="end">${esc(r.out.padStart(7))}</text>`,
      `<text x="${PAD + 320}" y="${yy}" fill="${muted}" font-family="${FONT}" font-size="13" text-anchor="end">${esc(r.trn.padStart(3))}</text>`,
      `<text x="${PAD + 335}" y="${yy}" fill="${blue}" font-family="${FONT}" font-size="13">${esc(r.model.padEnd(9))}</text>`,
      `<text x="${PAD + 410}" y="${yy}" fill="${muted}" font-family="${FONT}" font-size="12">${esc(r.when)}</text>`,
      `<text x="${PAD + 515}" y="${yy}" fill="${fg}" font-family="${FONT}" font-size="13">${esc(r.title)}</text>`,
    ].join("\n")
  })
  .join("\n")

const svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#12141a"/>
      <stop offset="100%" stop-color="#0e0e12"/>
    </linearGradient>
  </defs>
  <rect width="${W}" height="${H}" rx="12" fill="url(#bg)"/>
  <rect x="1" y="1" width="${W - 2}" height="${H - 2}" rx="11" fill="none" stroke="#3a3f50" stroke-width="1"/>

  <text x="${PAD}" y="28" font-family="${FONT}" font-size="14" font-weight="700" fill="#78d2ff">◆ gburn</text>
  <text x="${PAD + 85}" y="28" font-family="${FONT}" font-size="13" fill="#9aa0b0">grok build · api cost</text>
  <text x="${W - PAD}" y="28" font-family="${FONT}" font-size="12" fill="#9aa0b0" text-anchor="end">10 sessions</text>

  <text x="${PAD}" y="50" font-family="${FONT}" font-size="13">
    <tspan fill="#9aa0b0">COST </tspan><tspan fill="#64f0a0" font-weight="700">$226.75</tspan>
    <tspan fill="#3a3f50">  │  </tspan>
    <tspan fill="#9aa0b0">IN </tspan><tspan fill="#6eebfa">110.7M</tspan>
    <tspan fill="#3a3f50">  │  </tspan>
    <tspan fill="#9aa0b0">OUT </tspan><tspan fill="#dca0ff">917k</tspan>
    <tspan fill="#3a3f50">  │  </tspan>
    <tspan fill="#9aa0b0">OK </tspan><tspan fill="#ffd75a">10/10</tspan>
  </text>

  <line x1="${PAD}" y1="62" x2="${W - PAD}" y2="62" stroke="#3a3f50" stroke-width="1"/>

  <text x="${PAD + 22}" y="82" font-family="${FONT}" font-size="11" fill="#9aa0b0" font-weight="700">COST</text>
  <text x="${PAD + 165}" y="82" font-family="${FONT}" font-size="11" fill="#9aa0b0" font-weight="700">INPUT</text>
  <text x="${PAD + 235}" y="82" font-family="${FONT}" font-size="11" fill="#9aa0b0" font-weight="700">OUTPUT</text>
  <text x="${PAD + 300}" y="82" font-family="${FONT}" font-size="11" fill="#9aa0b0" font-weight="700">TRN</text>
  <text x="${PAD + 335}" y="82" font-family="${FONT}" font-size="11" fill="#9aa0b0" font-weight="700">MODEL</text>
  <text x="${PAD + 410}" y="82" font-family="${FONT}" font-size="11" fill="#9aa0b0" font-weight="700">WHEN</text>
  <text x="${PAD + 515}" y="82" font-family="${FONT}" font-size="11" fill="#9aa0b0" font-weight="700">TITLE</text>
  <line x1="${PAD}" y1="90" x2="${W - PAD}" y2="90" stroke="#2a2e3a" stroke-width="1"/>

  ${body}

  <rect x="${PAD}" y="${H - 110}" width="${W - PAD * 2}" height="72" rx="6" fill="#141820" stroke="#3a3f50"/>
  <text x="${PAD + 14}" y="${H - 85}" font-family="${FONT}" font-size="13">
    <tspan fill="#9aa0b0">▸ </tspan>
    <tspan fill="#ffffff" font-weight="700">Landing page redesign · conversion layout</tspan>
    <tspan fill="#ffd75a" font-weight="700">   $147.69</tspan>
  </text>
  <text x="${PAD + 14}" y="${H - 64}" font-family="${FONT}" font-size="12" fill="#9aa0b0">4.5 · 2026-07-09 17:52 · 78 turns · 710 tools · ~/dev/acme-landing</text>
  <text x="${PAD + 14}" y="${H - 44}" font-family="${FONT}" font-size="12">
    <tspan fill="#6eebfa">in 72.7M</tspan>
    <tspan fill="#dca0ff">  out 379k</tspan>
    <tspan fill="#9aa0b0">  · 195 streams · detailed · $2/$6 per 1M</tspan>
  </text>

  <text x="${PAD}" y="${H - 16}" font-family="${FONT}" font-size="12" fill="#9aa0b0">↑↓ move  ↵ open  / find  s sort  p price  m models  q quit</text>
  <text x="${W - PAD}" y="${H - 16}" font-family="${FONT}" font-size="12" fill="#9aa0b0" text-anchor="end">sort cost ↓</text>
</svg>
`

const out = join(outDir, "screenshot.svg")
writeFileSync(out, svg)
console.log("wrote", out)
