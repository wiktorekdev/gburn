export function formatTokens(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return "0"
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n >= 10_000_000 ? 1 : 2)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 100_000 ? 0 : 1)}k`
  return String(Math.round(n))
}

export function formatUsd(n: number, digits = 2): string {
  if (!Number.isFinite(n)) return "-"
  if (n === 0) return "$0.00"
  if (n > 0 && n < 0.01) return `$${n.toFixed(4)}`
  return `$${n.toFixed(digits)}`
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "-"
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return "-"
  const pad = (x: number) => String(x).padStart(2, "0")
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

export function formatDuration(seconds: number | null | undefined): string {
  if (!seconds || seconds <= 0) return "-"
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
}

export function truncate(s: string, max: number): string {
  if (s.length <= max) return s
  if (max <= 1) return "..."
  return s.slice(0, max - 1) + "..."
}

export function shortPath(p: string, max = 48): string {
  if (!p) return "-"
  const normalized = p.replace(/\//g, "\\")
  if (normalized.length <= max) return normalized
  const parts = normalized.split("\\").filter(Boolean)
  if (parts.length <= 2) return truncate(normalized, max)
  return truncate(`...\\${parts.slice(-2).join("\\")}`, max)
}

export function pad(s: string, width: number, align: "left" | "right" = "left"): string {
  const t = s.length > width ? truncate(s, width) : s
  if (align === "right") return t.padStart(width)
  return t.padEnd(width)
}

export function decodeCwdFolder(name: string): string {
  try {
    return decodeURIComponent(name)
  } catch {
    return name
  }
}
