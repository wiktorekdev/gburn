#!/usr/bin/env node
import { scanSessions } from "./scanner"
import { demoScan } from "./demo"
import { runTui, printSummary, printJson } from "./tui"

const NAME = "gburn"

function printHelp() {
  console.log(`${NAME} - Grok Build token burn at public API list prices

Usage:
  npx @wiktorekdev/gburn
  bunx @wiktorekdev/gburn
  ${NAME} [options]

Options:
  --summary, -s     Text summary (no TUI)
  --json, -j        JSON report
  --cwd <path>      Filter by project path
  --home <path>     Override GROK_HOME (default: ~/.grok)
  --help, -h        Show help

TUI keys:
  ↑↓ / j k   navigate     Enter   detail
  /          search       s       sort
  p          pricing      m       by model
  r          rescan       ?       help
  q          quit

Reads local sessions from ~/.grok/sessions
`)
}

async function main() {
  const args = process.argv.slice(2)

  if (args.includes("--help") || args.includes("-h")) {
    printHelp()
    return
  }

  const homeIdx = args.indexOf("--home")
  const grokHome = homeIdx >= 0 ? args[homeIdx + 1] : undefined
  const cwdIdx = args.indexOf("--cwd")
  const onlyCwd = cwdIdx >= 0 ? args[cwdIdx + 1] : undefined

  const wantJson = args.includes("--json") || args.includes("-j")
  const wantSummary = args.includes("--summary") || args.includes("-s")
  // hidden: GBURN_DEMO=1 → sample sessions for screenshots (not a public flag)
  const wantDemo = process.env.GBURN_DEMO === "1" || process.env.GBURN_DEMO === "true"

  const doScan = wantDemo
    ? async () => demoScan()
    : () => scanSessions({ grokHome, onlyCwd })

  const interactive =
    !wantJson &&
    !wantSummary &&
    Boolean(process.stdin.isTTY && process.stdout.isTTY)

  if (!wantJson && !interactive && !wantDemo) {
    console.error("Scanning Grok Build sessions…")
  }

  const scan = await doScan()

  if (wantJson) {
    printJson(scan)
    return
  }

  if (!interactive) {
    console.error(`Found ${scan.sessions.length} sessions`)
    printSummary(scan)
    return
  }

  await runTui(scan, doScan)
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
