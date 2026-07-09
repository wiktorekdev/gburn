import { join, basename } from "path"
import { readdir, readFile, stat } from "fs/promises"
import { createReadStream } from "fs"
import { createInterface } from "readline"

import {
  estimateFromUpdatesAsync,
  estimateFromSignals,
  type SessionUsage,
} from "./estimator"
import { decodeCwdFolder } from "./format"
import { calcCost } from "./pricing"

export type SessionRecord = {
  id: string
  dir: string
  cwd: string
  cwdFolder: string
  title: string
  modelId: string
  modelsUsed: string[]
  createdAt: string | null
  updatedAt: string | null
  lastActiveAt: string | null
  numMessages: number
  turnCount: number
  toolCallCount: number
  contextTokensUsed: number
  contextWindowTokens: number
  sessionDurationSeconds: number
  agentName: string | null
  reasoningEffort: string | null
  usage: SessionUsage
  /** Subagent linkage */
  isSubagent: boolean
  parentSessionId: string | null
  childSessionIds: string[]
  subagentType: string | null
  subagentDescription: string | null
}

export type ScanTotals = {
  sessions: number
  withDetailed: number
  inputTokens: number
  outputTokens: number
  totalTokens: number
  totalCost: number
  byModel: Map<string, { input: number; output: number; cost: number; sessions: number }>
}

export type ScanResult = {
  grokHome: string
  sessionsDir: string
  sessions: SessionRecord[]
  totals: ScanTotals
  scannedAt: string
}

type SubagentLink = {
  parentSessionId: string
  childSessionId: string
  subagentType: string | null
  description: string | null
}

function getGrokHome(): string {
  return process.env.GROK_HOME || join(process.env.USERPROFILE || process.env.HOME || "", ".grok")
}

async function pathExists(p: string): Promise<boolean> {
  try {
    await stat(p)
    return true
  } catch {
    return false
  }
}

async function isDir(p: string): Promise<boolean> {
  try {
    return (await stat(p)).isDirectory()
  } catch {
    return false
  }
}

async function readJson<T>(p: string): Promise<T | null> {
  try {
    const raw = await readFile(p, "utf8")
    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

/** Resolve project path: prefer .cwd file (slug folders), else URL-decode folder name. */
async function resolveCwd(folderPath: string, folderName: string): Promise<string> {
  const cwdFile = join(folderPath, ".cwd")
  try {
    const raw = (await readFile(cwdFile, "utf8")).trim()
    if (raw) return raw
  } catch {
    /* no .cwd */
  }
  return decodeCwdFolder(folderName)
}

type SummaryJson = {
  info?: { id?: string; cwd?: string }
  session_summary?: string
  generated_title?: string
  created_at?: string
  updated_at?: string
  last_active_at?: string
  num_messages?: number
  current_model_id?: string
  agent_name?: string
  reasoning_effort?: string
}

type SignalsJson = {
  turnCount?: number
  toolCallCount?: number
  contextTokensUsed?: number
  contextWindowTokens?: number
  sessionDurationSeconds?: number
  modelsUsed?: string[]
  primaryModelId?: string
}

type SubagentMetaJson = {
  subagent_id?: string
  parent_session_id?: string
  child_session_id?: string
  subagent_type?: string
  description?: string
}

/** Prefer model_id from actual assistant messages — signals.primaryModelId can be wrong. */
async function resolveModelFromChat(chatPath: string): Promise<{
  primary: string | null
  counts: Map<string, number>
}> {
  const counts = new Map<string, number>()
  try {
    const rl = createInterface({
      input: createReadStream(chatPath, { encoding: "utf8" }),
      crlfDelay: Infinity,
    })
    for await (const line of rl) {
      if (!line.includes("model_id")) continue
      try {
        const o = JSON.parse(line) as { type?: string; model_id?: string }
        if (o.type === "assistant" && typeof o.model_id === "string" && o.model_id) {
          counts.set(o.model_id, (counts.get(o.model_id) ?? 0) + 1)
        }
      } catch {
        /* skip */
      }
    }
  } catch {
    return { primary: null, counts }
  }

  let best: string | null = null
  let bestN = 0
  for (const [id, n] of counts) {
    if (n > bestN) {
      best = id
      bestN = n
    }
  }
  return { primary: best, counts }
}

function resolveModelId(opts: {
  fromChat: string | null
  currentModelId?: string
  primaryModelId?: string
  modelsUsed?: string[]
}): string {
  if (opts.fromChat) return opts.fromChat
  if (opts.currentModelId) return opts.currentModelId
  if (opts.primaryModelId) return opts.primaryModelId
  if (opts.modelsUsed?.[0]) return opts.modelsUsed[0]
  return "unknown"
}

/** Index parent↔child links from session subagents/meta.json files. */
async function indexSubagents(sessionsDir: string): Promise<{
  byChild: Map<string, SubagentLink>
  childrenOf: Map<string, string[]>
}> {
  const byChild = new Map<string, SubagentLink>()
  const childrenOf = new Map<string, string[]>()

  let cwdFolders: string[]
  try {
    const entries = await readdir(sessionsDir, { withFileTypes: true })
    cwdFolders = entries.filter((e) => e.isDirectory()).map((e) => e.name)
  } catch {
    return { byChild, childrenOf }
  }

  for (const folder of cwdFolders) {
    const folderPath = join(sessionsDir, folder)
    let sessionDirs: string[]
    try {
      const entries = await readdir(folderPath, { withFileTypes: true })
      sessionDirs = entries.filter((e) => e.isDirectory()).map((e) => e.name)
    } catch {
      continue
    }

    for (const sessionId of sessionDirs) {
      const subRoot = join(folderPath, sessionId, "subagents")
      if (!(await isDir(subRoot))) continue

      let childDirs: string[]
      try {
        const entries = await readdir(subRoot, { withFileTypes: true })
        childDirs = entries.filter((e) => e.isDirectory()).map((e) => e.name)
      } catch {
        continue
      }

      for (const childDir of childDirs) {
        const meta = await readJson<SubagentMetaJson>(join(subRoot, childDir, "meta.json"))
        if (!meta) continue
        const parent = meta.parent_session_id || sessionId
        const child = meta.child_session_id || meta.subagent_id || childDir
        if (!parent || !child) continue

        const link: SubagentLink = {
          parentSessionId: parent,
          childSessionId: child,
          subagentType: meta.subagent_type ?? null,
          description: meta.description ?? null,
        }
        byChild.set(child, link)
        const list = childrenOf.get(parent) ?? []
        if (!list.includes(child)) list.push(child)
        childrenOf.set(parent, list)
      }
    }
  }

  return { byChild, childrenOf }
}

async function loadSession(
  sessionDir: string,
  cwdFolder: string,
  resolvedCwd: string,
  subagents: { byChild: Map<string, SubagentLink>; childrenOf: Map<string, string[]> },
): Promise<SessionRecord | null> {
  const summary = await readJson<SummaryJson>(join(sessionDir, "summary.json"))
  if (!summary) return null

  const signals = await readJson<SignalsJson>(join(sessionDir, "signals.json"))
  const id = summary.info?.id || basename(sessionDir)
  const cwd = summary.info?.cwd || resolvedCwd

  const chatModels = await resolveModelFromChat(join(sessionDir, "chat_history.jsonl"))
  const modelId = resolveModelId({
    fromChat: chatModels.primary,
    currentModelId: summary.current_model_id,
    primaryModelId: signals?.primaryModelId,
    modelsUsed: signals?.modelsUsed,
  })

  const modelsUsed =
    chatModels.counts.size > 0
      ? [...chatModels.counts.entries()]
          .sort((a, b) => b[1] - a[1])
          .map(([m]) => m)
      : signals?.modelsUsed?.length
        ? signals.modelsUsed
        : [modelId]

  const title =
    summary.generated_title ||
    summary.session_summary ||
    id.slice(0, 8)

  let usage = await estimateFromUpdatesAsync(
    join(sessionDir, "updates.jsonl"),
    modelId,
    chatModels.counts.size > 0 ? chatModels.counts : undefined,
  )

  if (!usage) {
    const ctx = signals?.contextTokensUsed ?? 0
    const turns = signals?.turnCount ?? 0
    if (ctx > 0 || turns > 0) {
      usage = estimateFromSignals(
        ctx,
        turns || 1,
        modelId,
        chatModels.counts.size > 0 ? chatModels.counts : undefined,
      )
    } else {
      usage = {
        inputTokens: 0,
        outputTokens: 0,
        totalTokens: 0,
        streams: 0,
        hasDetailedUsage: false,
        streamsDetail: [],
        byModel: [],
        cost: calcCost(modelId, 0, 0),
        estimationNote: "No usage data found for this session.",
      }
    }
  }

  const asChild = subagents.byChild.get(id)
  const childIds = subagents.childrenOf.get(id) ?? []

  return {
    id,
    dir: sessionDir,
    cwd,
    cwdFolder,
    title,
    modelId,
    modelsUsed,
    createdAt: summary.created_at ?? null,
    updatedAt: summary.updated_at ?? null,
    lastActiveAt: summary.last_active_at ?? summary.updated_at ?? null,
    numMessages: summary.num_messages ?? 0,
    turnCount: signals?.turnCount ?? 0,
    toolCallCount: signals?.toolCallCount ?? 0,
    contextTokensUsed: signals?.contextTokensUsed ?? 0,
    contextWindowTokens: signals?.contextWindowTokens ?? 0,
    sessionDurationSeconds: signals?.sessionDurationSeconds ?? 0,
    agentName: summary.agent_name ?? null,
    reasoningEffort: summary.reasoning_effort ?? null,
    usage,
    isSubagent: Boolean(asChild),
    parentSessionId: asChild?.parentSessionId ?? null,
    childSessionIds: childIds,
    subagentType: asChild?.subagentType ?? null,
    subagentDescription: asChild?.description ?? null,
  }
}

export async function scanSessions(opts?: {
  grokHome?: string
  onlyCwd?: string
}): Promise<ScanResult> {
  const grokHome = opts?.grokHome || getGrokHome()
  const sessionsDir = join(grokHome, "sessions")
  const sessions: SessionRecord[] = []

  if (!(await isDir(sessionsDir))) {
    return {
      grokHome,
      sessionsDir,
      sessions: [],
      totals: emptyTotals(),
      scannedAt: new Date().toISOString(),
    }
  }

  const subagents = await indexSubagents(sessionsDir)

  let cwdFolders: string[]
  try {
    const entries = await readdir(sessionsDir, { withFileTypes: true })
    cwdFolders = entries.filter((e) => e.isDirectory()).map((e) => e.name)
  } catch {
    cwdFolders = []
  }

  for (const folder of cwdFolders) {
    const folderPath = join(sessionsDir, folder)
    const resolvedCwd = await resolveCwd(folderPath, folder)

    let children: string[]
    try {
      const entries = await readdir(folderPath, { withFileTypes: true })
      children = entries.filter((e) => e.isDirectory()).map((e) => e.name)
    } catch {
      continue
    }

    for (const sessionId of children) {
      const sessionDir = join(folderPath, sessionId)
      if (!(await pathExists(join(sessionDir, "summary.json")))) continue

      const rec = await loadSession(sessionDir, folder, resolvedCwd, subagents)
      if (!rec) continue

      if (opts?.onlyCwd) {
        const want = opts.onlyCwd.toLowerCase()
        if (
          !rec.cwd.toLowerCase().includes(want) &&
          !resolvedCwd.toLowerCase().includes(want) &&
          !decodeCwdFolder(folder).toLowerCase().includes(want)
        ) {
          continue
        }
      }

      if (!rec.usage.cost.priced) continue
      if (rec.usage.inputTokens <= 0 && rec.usage.outputTokens <= 0) continue

      sessions.push(rec)
    }
  }

  sessions.sort((a, b) => {
    const ta = a.lastActiveAt ? Date.parse(a.lastActiveAt) : 0
    const tb = b.lastActiveAt ? Date.parse(b.lastActiveAt) : 0
    return tb - ta
  })

  return {
    grokHome,
    sessionsDir,
    sessions,
    totals: computeTotals(sessions),
    scannedAt: new Date().toISOString(),
  }
}

function emptyTotals(): ScanTotals {
  return {
    sessions: 0,
    withDetailed: 0,
    inputTokens: 0,
    outputTokens: 0,
    totalTokens: 0,
    totalCost: 0,
    byModel: new Map(),
  }
}

export function computeTotals(sessions: SessionRecord[]): ScanTotals {
  const totals = emptyTotals()
  totals.sessions = sessions.length

  for (const s of sessions) {
    if (s.usage.hasDetailedUsage) totals.withDetailed++
    totals.inputTokens += s.usage.inputTokens
    totals.outputTokens += s.usage.outputTokens
    totals.totalTokens += s.usage.totalTokens
    totals.totalCost += s.usage.cost.totalCost

    // prefer per-model split when multi-model
    if (s.usage.byModel.length > 0) {
      for (const m of s.usage.byModel) {
        const row = totals.byModel.get(m.modelId) ?? {
          input: 0,
          output: 0,
          cost: 0,
          sessions: 0,
        }
        row.input += m.inputTokens
        row.output += m.outputTokens
        row.cost += m.cost.totalCost
        row.sessions++
        totals.byModel.set(m.modelId, row)
      }
    } else {
      const mid = s.modelId
      const row = totals.byModel.get(mid) ?? { input: 0, output: 0, cost: 0, sessions: 0 }
      row.input += s.usage.inputTokens
      row.output += s.usage.outputTokens
      row.cost += s.usage.cost.totalCost
      row.sessions++
      totals.byModel.set(mid, row)
    }
  }

  return totals
}

export type SortKey = "cost" | "date" | "input" | "output" | "turns" | "title"

export function sortSessions(sessions: SessionRecord[], key: SortKey, desc = true): SessionRecord[] {
  const mul = desc ? -1 : 1
  const sorted = [...sessions]
  sorted.sort((a, b) => {
    let cmp = 0
    switch (key) {
      case "cost":
        cmp = a.usage.cost.totalCost - b.usage.cost.totalCost
        break
      case "date": {
        const ta = a.lastActiveAt ? Date.parse(a.lastActiveAt) : 0
        const tb = b.lastActiveAt ? Date.parse(b.lastActiveAt) : 0
        cmp = ta - tb
        break
      }
      case "input":
        cmp = a.usage.inputTokens - b.usage.inputTokens
        break
      case "output":
        cmp = a.usage.outputTokens - b.usage.outputTokens
        break
      case "turns":
        cmp = a.turnCount - b.turnCount
        break
      case "title":
        cmp = a.title.localeCompare(b.title)
        break
    }
    return cmp * mul
  })
  return sorted
}
