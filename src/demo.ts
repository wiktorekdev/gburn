import type { ScanResult, SessionRecord } from "./scanner"
import { computeTotals } from "./scanner"
import { calcCost } from "./pricing"

function session( partial: {
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
    agentName: "grok-build-plan",
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
    subagentDescription: partial.isSubagent ? "demo subagent" : null,
  }
}

/** Fake sessions for screenshots / demos — no real paths or usernames. */
export function demoScan(): ScanResult {
  const parentId = "demo-parent-web-port"
  const sessions: SessionRecord[] = [
    session({
      id: "demo-landing-redesign",
      title: "Landing page redesign · conversion layout",
      cwd: "~/dev/acme-landing",
      modelId: "grok-4.5",
      input: 72_700_000,
      output: 379_000,
      turns: 78,
      tools: 710,
      when: "2026-07-09T17:52:00Z",
    }),
    session({
      id: "demo-api-dashboard",
      title: "Internal metrics dashboard + charts",
      cwd: "~/dev/ops-dashboard",
      modelId: "grok-4.5",
      input: 15_800_000,
      output: 161_000,
      turns: 18,
      tools: 140,
      when: "2026-07-09T21:34:00Z",
    }),
    session({
      id: parentId,
      title: "Game web port · runtime fidelity [2 sub]",
      cwd: "~/dev/game-web-port",
      modelId: "grok-4.5",
      input: 11_800_000,
      output: 140_000,
      turns: 12,
      tools: 95,
      when: "2026-07-09T20:55:00Z",
      childSessionIds: ["demo-sub-re", "demo-sub-runtime"],
    }),
    session({
      id: "demo-pr-review",
      title: "PR review pass on auth middleware",
      cwd: "~/dev/saas-api",
      modelId: "grok-4.5",
      input: 6_330_000,
      output: 59_700,
      turns: 7,
      tools: 42,
      when: "2026-07-09T18:51:00Z",
    }),
    session({
      id: "demo-sub-re",
      title: "↳ Asset extraction pass",
      cwd: "~/dev/game-web-port",
      modelId: "grok-4.5",
      input: 2_310_000,
      output: 74_700,
      turns: 1,
      tools: 28,
      when: "2026-07-09T20:41:00Z",
      isSubagent: true,
      parentSessionId: parentId,
    }),
    session({
      id: "demo-sub-runtime",
      title: "↳ Runtime polish + input mapping",
      cwd: "~/dev/game-web-port",
      modelId: "grok-4.5",
      input: 1_170_000,
      output: 61_200,
      turns: 1,
      tools: 19,
      when: "2026-07-09T20:39:00Z",
      isSubagent: true,
      parentSessionId: parentId,
    }),
    session({
      id: "demo-cli-tool",
      title: "Scaffold CLI for log parsing",
      cwd: "~/dev/logparse-cli",
      modelId: "grok-4.5",
      input: 490_000,
      output: 15_300,
      turns: 5,
      tools: 22,
      when: "2026-07-09T13:48:00Z",
    }),
    session({
      id: "demo-ideas",
      title: "Brainstorm side-project ideas",
      cwd: "~/notes",
      modelId: "grok-4.5",
      input: 26_000,
      output: 10_100,
      turns: 1,
      tools: 0,
      when: "2026-07-09T13:41:00Z",
    }),
    session({
      id: "demo-dns-fix",
      title: "Debug DNS on home lab DNS box",
      cwd: "~/infra/homelab",
      modelId: "grok-build",
      input: 49_600,
      output: 11_800,
      turns: 3,
      tools: 12,
      when: "2026-07-09T14:36:00Z",
    }),
    session({
      id: "demo-composer",
      title: "Quick edit on README badges",
      cwd: "~/dev/docs-site",
      modelId: "grok-composer-2.5-fast",
      input: 10_000,
      output: 2_400,
      turns: 1,
      tools: 3,
      when: "2026-07-09T17:53:00Z",
    }),
  ]

  // strip [2 sub] from title storage — display uses childSessionIds
  const fixed = sessions.map((s) =>
    s.id === parentId
      ? { ...s, title: "Game web port · runtime fidelity" }
      : s,
  )

  return {
    grokHome: "~/.grok",
    sessionsDir: "~/.grok/sessions",
    sessions: fixed,
    totals: computeTotals(fixed),
    scannedAt: "2026-07-09T21:40:00.000Z",
  }
}
