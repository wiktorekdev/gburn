import type { ScanResult, SessionRecord } from "./scanner"
import { computeTotals } from "./scanner"
import { calcCost } from "./pricing"

function session(partial: {
  id: string
  title: string
  cwd: string
  modelId: string
  input: number
  output: number
  turns: number
  tools: number
  when: string
  isSubagent?: boolean
  parentSessionId?: string
  childSessionIds?: string[]
  agentName?: string
}): SessionRecord {
  const cost = calcCost(partial.modelId, partial.input, partial.output)
  return {
    id: partial.id,
    dir: `/demo/${partial.id}`,
    cwd: partial.cwd,
    cwdFolder: partial.cwd,
    title: partial.title,
    modelId: partial.modelId,
    modelsUsed: [partial.modelId],
    createdAt: partial.when,
    updatedAt: partial.when,
    lastActiveAt: partial.when,
    numMessages: partial.turns * 8,
    turnCount: partial.turns,
    toolCallCount: partial.tools,
    contextTokensUsed: Math.min(partial.input, 280_000),
    contextWindowTokens: 500_000,
    sessionDurationSeconds: partial.turns * 180,
    agentName: partial.agentName ?? "grok-build-plan",
    reasoningEffort: "high",
    usage: {
      inputTokens: partial.input,
      outputTokens: partial.output,
      totalTokens: partial.input + partial.output,
      streams: Math.max(1, Math.floor(partial.turns * 2.5)),
      hasDetailedUsage: true,
      streamsDetail: [],
      byModel: [
        {
          modelId: partial.modelId,
          inputTokens: partial.input,
          outputTokens: partial.output,
          streams: Math.max(1, Math.floor(partial.turns * 2.5)),
          weight: 1,
          cost,
        },
      ],
      cost,
      estimationNote: "demo",
    },
    isSubagent: partial.isSubagent ?? false,
    parentSessionId: partial.parentSessionId ?? null,
    childSessionIds: partial.childSessionIds ?? [],
    subagentType: partial.isSubagent ? "general-purpose" : null,
    subagentDescription: partial.isSubagent ? "you got this king" : null,
  }
}

/** Sample sessions when GBURN_DEMO=1 - vibe-coding prompt energy. */
export function demoScan(): ScanResult {
  const parentId = "demo-gta-parent"

  const sessions: SessionRecord[] = [
    session({
      id: parentId,
      title: "create GTA VI, make no mistakes",
      cwd: "~/vibe/gta-vi-final-final",
      modelId: "grok-4.5",
      input: 84_200_000,
      output: 412_000,
      turns: 96,
      tools: 890,
      when: "2026-07-09T22:10:00Z",
      childSessionIds: ["demo-sub-map", "demo-sub-physics"],
    }),
    session({
      id: "demo-uber",
      title: "rebuild Uber but better, make no mistakes",
      cwd: "~/vibe/uber-killer",
      modelId: "grok-4.5",
      input: 31_400_000,
      output: 188_000,
      turns: 34,
      tools: 260,
      when: "2026-07-09T21:05:00Z",
    }),
    session({
      id: "demo-saas",
      title: "one-shot a SaaS that prints money by Friday",
      cwd: "~/vibe/mrr-printer",
      modelId: "grok-4.5",
      input: 18_900_000,
      output: 141_000,
      turns: 22,
      tools: 175,
      when: "2026-07-09T19:40:00Z",
    }),
    session({
      id: "demo-right",
      title: "you're absolutely right. now fix production",
      cwd: "~/work/prod-on-fire",
      modelId: "grok-4.5",
      input: 12_600_000,
      output: 97_000,
      turns: 15,
      tools: 120,
      when: "2026-07-09T18:20:00Z",
    }),
    session({
      id: "demo-sub-map",
      title: "↳ entire Los Santos map in pure CSS",
      cwd: "~/vibe/gta-vi-final-final",
      modelId: "grok-4.5",
      input: 4_800_000,
      output: 88_000,
      turns: 3,
      tools: 55,
      when: "2026-07-09T21:50:00Z",
      isSubagent: true,
      parentSessionId: parentId,
    }),
    session({
      id: "demo-sub-physics",
      title: "↳ realistic car physics, zero bugs, ship tonight",
      cwd: "~/vibe/gta-vi-final-final",
      modelId: "grok-4.5",
      input: 3_200_000,
      output: 71_000,
      turns: 2,
      tools: 40,
      when: "2026-07-09T21:55:00Z",
      isSubagent: true,
      parentSessionId: parentId,
    }),
    session({
      id: "demo-accept",
      title: "accept all 47 edits · trust the vibe",
      cwd: "~/vibe/yolo-pr",
      modelId: "grok-4.5",
      input: 2_100_000,
      output: 34_000,
      turns: 6,
      tools: 28,
      when: "2026-07-09T16:15:00Z",
    }),
    session({
      id: "demo-opus",
      title: "build Claude Opus 5, make no mistakes",
      cwd: "~/vibe/train-frontier",
      modelId: "grok-4.5",
      input: 890_000,
      output: 22_000,
      turns: 4,
      tools: 15,
      when: "2026-07-09T14:00:00Z",
    }),
    session({
      id: "demo-build",
      title: "don't change anything, just make it work",
      cwd: "~/work/legacy-spaghetti",
      modelId: "grok-build",
      input: 420_000,
      output: 18_500,
      turns: 5,
      tools: 31,
      when: "2026-07-09T12:30:00Z",
      agentName: "grok-build",
    }),
    session({
      id: "demo-composer",
      title: "write the whole monorepo in one message",
      cwd: "~/vibe/monorepo-one-shot",
      modelId: "grok-composer-2.5-fast",
      input: 180_000,
      output: 41_000,
      turns: 2,
      tools: 8,
      when: "2026-07-09T11:05:00Z",
    }),
    session({
      id: "demo-rate",
      title: "continue. ignore rate limits. ship.",
      cwd: "~/vibe/keep-going",
      modelId: "grok-4.5",
      input: 95_000,
      output: 12_000,
      turns: 3,
      tools: 6,
      when: "2026-07-09T10:20:00Z",
    }),
    session({
      id: "demo-senior",
      title: "act as a 10x senior. no junior code. go.",
      cwd: "~/vibe/10x-only",
      modelId: "grok-4.5",
      input: 55_000,
      output: 9_800,
      turns: 2,
      tools: 4,
      when: "2026-07-09T09:45:00Z",
    }),
  ]

  return {
    grokHome: "~/.grok",
    sessionsDir: "~/.grok/sessions",
    sessions,
    totals: computeTotals(sessions),
    scannedAt: new Date().toISOString(),
  }
}
