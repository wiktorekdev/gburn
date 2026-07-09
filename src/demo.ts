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
    subagentDescription: partial.isSubagent ? "nested chaos" : null,
  }
}

/** Sample sessions for GBURN_DEMO=1 — funny titles, no real paths. */
export function demoScan(): ScanResult {
  const parentId = "demo-nest-parent"
  const sessions: SessionRecord[] = [
    session({
      id: "demo-hello",
      title: 'helloworld("print") — why is this in production',
      cwd: "~/dev/hello-again",
      modelId: "grok-4.5",
      input: 72_700_000,
      output: 379_000,
      turns: 78,
      tools: 710,
      when: "2026-07-09T17:52:00Z",
    }),
    session({
      id: "demo-antigravity",
      title: "import antigravity  # it worked on my machine",
      cwd: "~/dev/space-cadet",
      modelId: "grok-4.5",
      input: 15_800_000,
      output: 161_000,
      turns: 18,
      tools: 140,
      when: "2026-07-09T21:34:00Z",
    }),
    session({
      id: parentId,
      title: "refactor while true: coffee()  [2 sub]",
      cwd: "~/dev/caffeine-os",
      modelId: "grok-4.5",
      input: 11_800_000,
      output: 140_000,
      turns: 12,
      tools: 95,
      when: "2026-07-09T20:55:00Z",
      childSessionIds: ["demo-sub-1", "demo-sub-2"],
    }),
    session({
      id: "demo-friday",
      title: "deploy to prod on friday (send help)",
      cwd: "~/dev/yolo-ship",
      modelId: "grok-4.5",
      input: 6_330_000,
      output: 59_700,
      turns: 7,
      tools: 42,
      when: "2026-07-09T18:51:00Z",
    }),
    session({
      id: "demo-sub-1",
      title: "↳ undefined is not a function (again)",
      cwd: "~/dev/caffeine-os",
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
      id: "demo-sub-2",
      title: "↳ CSS is awesome (center the div)",
      cwd: "~/dev/caffeine-os",
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
      id: "demo-git",
      title: 'git commit -m "fix"  (it was not a fix)',
      cwd: "~/dev/honest-commits",
      modelId: "grok-4.5",
      input: 490_000,
      output: 15_300,
      turns: 5,
      tools: 22,
      when: "2026-07-09T13:48:00Z",
    }),
    session({
      id: "demo-todo",
      title: "TODO: delete this TODO",
      cwd: "~/notes",
      modelId: "grok-4.5",
      input: 26_000,
      output: 10_100,
      turns: 1,
      tools: 0,
      when: "2026-07-09T13:41:00Z",
    }),
    session({
      id: "demo-build",
      title: "sudo make me a sandwich",
      cwd: "~/infra/homelab",
      modelId: "grok-build",
      input: 49_600,
      output: 11_800,
      turns: 3,
      tools: 12,
      when: "2026-07-09T14:36:00Z",
      agentName: "grok-build",
    }),
    session({
      id: "demo-composer",
      title: "console.log('why') // still here",
      cwd: "~/dev/docs-site",
      modelId: "grok-composer-2.5-fast",
      input: 10_000,
      output: 2_400,
      turns: 1,
      tools: 3,
      when: "2026-07-09T17:53:00Z",
    }),
  ]

  const fixed = sessions.map((s) =>
    s.id === parentId
      ? { ...s, title: "refactor while true: coffee()" }
      : s,
  )

  return {
    grokHome: "~/.grok",
    sessionsDir: "~/.grok/sessions",
    sessions: fixed,
    totals: computeTotals(fixed),
    scannedAt: new Date().toISOString(),
  }
}
