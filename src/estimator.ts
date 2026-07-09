import { createReadStream } from "fs"
import { createInterface } from "readline"

import { calcCost, type CostBreakdown } from "./pricing"

export type StreamUsage = {
  streamStartMs: number | null
  inputTokens: number
  outputTokens: number
  updateTypes: string[]
}

export type ModelUsage = {
  modelId: string
  inputTokens: number
  outputTokens: number
  streams: number
  weight: number
  cost: CostBreakdown
}

export type SessionUsage = {
  inputTokens: number
  outputTokens: number
  totalTokens: number
  streams: number
  hasDetailedUsage: boolean
  streamsDetail: StreamUsage[]
  byModel: ModelUsage[]
  cost: CostBreakdown
  estimationNote: string
}

const AGENT_TYPES = new Set([
  "AgentThoughtChunk",
  "AgentMessageChunk",
  "ToolCall",
  "Plan",
  "agent_thought_chunk",
  "agent_message_chunk",
  "tool_call",
  "plan",
])

type StreamEvent = {
  tt: number
  type: string
  streamStartMs: number | null
}

function extractMeta(obj: Record<string, unknown>): {
  totalTokens: number
  updateType: string
  streamStartMs: number | null
} | null {
  const params = obj.params as Record<string, unknown> | undefined
  if (!params) return null
  const meta = params._meta as Record<string, unknown> | undefined
  if (!meta || typeof meta.totalTokens !== "number") return null

  const update = params.update as Record<string, unknown> | undefined
  const sessionUpdate = typeof update?.sessionUpdate === "string" ? update.sessionUpdate : ""
  const updateType = typeof meta.updateType === "string" ? meta.updateType : sessionUpdate

  return {
    totalTokens: meta.totalTokens,
    updateType,
    streamStartMs: typeof meta.streamStartMs === "number" ? meta.streamStartMs : null,
  }
}

/** Stream-parse updates.jsonl (line by line — big sessions can be multi‑MB). */
export async function estimateFromUpdatesAsync(
  updatesPath: string,
  primaryModelId: string | null,
  modelWeights?: Map<string, number>,
): Promise<SessionUsage | null> {
  const byStream = new Map<string, StreamEvent[]>()
  let fallbackIdx = 0
  let sawTokens = false

  try {
    const rl = createInterface({
      input: createReadStream(updatesPath, { encoding: "utf8" }),
      crlfDelay: Infinity,
    })

    for await (const line of rl) {
      if (!line.includes("totalTokens")) continue
      let obj: Record<string, unknown>
      try {
        obj = JSON.parse(line)
      } catch {
        continue
      }
      const meta = extractMeta(obj)
      if (!meta) continue
      sawTokens = true

      const key =
        meta.streamStartMs != null ? String(meta.streamStartMs) : `anon-${fallbackIdx++}`
      const list = byStream.get(key) ?? []
      list.push({
        tt: meta.totalTokens,
        type: meta.updateType,
        streamStartMs: meta.streamStartMs,
      })
      byStream.set(key, list)
    }
  } catch {
    return null
  }

  if (!sawTokens || byStream.size === 0) return null

  const streamsDetail: StreamUsage[] = []
  let inputTokens = 0
  let outputTokens = 0

  for (const evs of byStream.values()) {
    if (evs.length === 0) continue
    const first = evs[0].tt
    const types = [...new Set(evs.map((e) => e.type).filter(Boolean))]

    const agentTts = evs.filter((e) => AGENT_TYPES.has(e.type)).map((e) => e.tt)
    let out = 0
    if (agentTts.length > 0) {
      out = Math.max(0, Math.max(...agentTts) - first)
    } else {
      const raw = Math.max(...evs.map((e) => e.tt)) - first
      if (raw > 0) out = Math.floor(raw * 0.15)
    }

    inputTokens += first
    outputTokens += out
    streamsDetail.push({
      streamStartMs: evs[0].streamStartMs,
      inputTokens: first,
      outputTokens: out,
      updateTypes: types,
    })
  }

  return finalizeUsage({
    inputTokens,
    outputTokens,
    streams: streamsDetail.length,
    streamsDetail,
    hasDetailedUsage: true,
    primaryModelId,
    modelWeights,
    estimationNote:
      "Estimated from updates.jsonl totalTokens per stream (input @ stream start, output ≈ agent growth). Multi-model sessions split by chat_history message share. Not official billing.",
  })
}

/** Fallback when updates.jsonl has no totalTokens. */
export function estimateFromSignals(
  contextTokensUsed: number,
  turnCount: number,
  primaryModelId: string | null,
  modelWeights?: Map<string, number>,
): SessionUsage {
  const avgInput = contextTokensUsed > 0 ? Math.floor(contextTokensUsed * 0.55) : 0
  const turns = Math.max(turnCount, 1)
  const inputTokens = avgInput * turns

  return finalizeUsage({
    inputTokens,
    outputTokens: 0,
    streams: turns,
    streamsDetail: [],
    hasDetailedUsage: false,
    primaryModelId,
    modelWeights,
    estimationNote:
      "Rough fallback from signals.json (peak context × turns). Output unknown — lower bound. Multi-model split by chat share when available.",
  })
}

function finalizeUsage(opts: {
  inputTokens: number
  outputTokens: number
  streams: number
  streamsDetail: StreamUsage[]
  hasDetailedUsage: boolean
  primaryModelId: string | null
  modelWeights?: Map<string, number>
  estimationNote: string
}): SessionUsage {
  const byModel = splitByModel(
    opts.inputTokens,
    opts.outputTokens,
    opts.streams,
    opts.primaryModelId,
    opts.modelWeights,
  )

  const totalCost = byModel.reduce((s, m) => s + m.cost.totalCost, 0)
  const totalInCost = byModel.reduce((s, m) => s + m.cost.inputCost, 0)
  const totalOutCost = byModel.reduce((s, m) => s + m.cost.outputCost, 0)
  const anyPriced = byModel.some((m) => m.cost.priced)
  const primary = opts.primaryModelId ?? byModel[0]?.modelId ?? "unknown"
  const primaryPrice = calcCost(primary, opts.inputTokens, opts.outputTokens)

  return {
    inputTokens: opts.inputTokens,
    outputTokens: opts.outputTokens,
    totalTokens: opts.inputTokens + opts.outputTokens,
    streams: opts.streams,
    hasDetailedUsage: opts.hasDetailedUsage,
    streamsDetail: opts.streamsDetail,
    byModel,
    cost: {
      inputCost: totalInCost,
      outputCost: totalOutCost,
      totalCost,
      priced: anyPriced,
      price: primaryPrice.price,
    },
    estimationNote: opts.estimationNote,
  }
}

/** Split session tokens across models by chat_history message weights. */
export function splitByModel(
  inputTokens: number,
  outputTokens: number,
  streams: number,
  primaryModelId: string | null,
  modelWeights?: Map<string, number>,
): ModelUsage[] {
  const weights = new Map<string, number>()

  if (modelWeights && modelWeights.size > 0) {
    for (const [id, w] of modelWeights) {
      if (w > 0) weights.set(id, w)
    }
  }

  if (weights.size === 0) {
    const id = primaryModelId ?? "unknown"
    weights.set(id, 1)
  }

  let totalW = 0
  for (const w of weights.values()) totalW += w
  if (totalW <= 0) {
    const id = primaryModelId ?? "unknown"
    const cost = calcCost(id, inputTokens, outputTokens)
    return [
      {
        modelId: id,
        inputTokens,
        outputTokens,
        streams,
        weight: 1,
        cost,
      },
    ]
  }

  const entries = [...weights.entries()]
  const result: ModelUsage[] = []
  let assignedIn = 0
  let assignedOut = 0
  let assignedStreams = 0

  for (let i = 0; i < entries.length; i++) {
    const [modelId, w] = entries[i]
    const share = w / totalW
    const isLast = i === entries.length - 1

    // last model gets remainder so we don't lose tokens to rounding
    const inTok = isLast
      ? inputTokens - assignedIn
      : Math.floor(inputTokens * share)
    const outTok = isLast
      ? outputTokens - assignedOut
      : Math.floor(outputTokens * share)
    const streamN = isLast
      ? Math.max(0, streams - assignedStreams)
      : Math.floor(streams * share)

    assignedIn += inTok
    assignedOut += outTok
    assignedStreams += streamN

    result.push({
      modelId,
      inputTokens: inTok,
      outputTokens: outTok,
      streams: streamN,
      weight: share,
      cost: calcCost(modelId, inTok, outTok),
    })
  }

  // sort by cost desc
  result.sort((a, b) => b.cost.totalCost - a.cost.totalCost)
  return result
}
