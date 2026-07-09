# gburn

<p align="center">
  <strong>Grok Build burn rate</strong> — see how much your sessions would cost at public API list prices.
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@wiktorekdev/gburn"><img src="https://img.shields.io/npm/v/@wiktorekdev/gburn.svg?style=flat-square&color=cb3837" alt="npm version" /></a>
  <a href="https://www.npmjs.com/package/@wiktorekdev/gburn"><img src="https://img.shields.io/npm/dm/@wiktorekdev/gburn.svg?style=flat-square&color=blue" alt="npm downloads" /></a>
  <a href="https://github.com/wiktorekdev/gburn/blob/main/LICENSE"><img src="https://img.shields.io/npm/l/@wiktorekdev/gburn.svg?style=flat-square" alt="license" /></a>
  <a href="https://nodejs.org"><img src="https://img.shields.io/node/v/@wiktorekdev/gburn.svg?style=flat-square&color=brightgreen" alt="node" /></a>
  <a href="https://github.com/wiktorekdev/gburn"><img src="https://img.shields.io/github/stars/wiktorekdev/gburn?style=flat-square" alt="stars" /></a>
</p>

<p align="center">
  <code>npx @wiktorekdev/gburn</code> &nbsp;·&nbsp; <code>bunx @wiktorekdev/gburn</code>
</p>

---

Fullscreen TUI that reads **local** Grok Build sessions (`~/.grok/sessions`) and estimates token spend + list-price cost for:

| Model | Typical region | List price (in / out per 1M) |
|-------|----------------|------------------------------|
| **Grok 4.5** | USA default | $2 / $6 (xAI) |
| **Composer 2.5 Fast** | rest of world default | $3 / $15 (Cursor) |
| **grok-build** | some regions (e.g. EU) | $1 / $2 (xAI Code API) |

Nothing is uploaded. No account required. Zero runtime dependencies.

## Install

```bash
# one-shot
npx @wiktorekdev/gburn
bunx @wiktorekdev/gburn

# global (CLI command: gburn)
npm i -g @wiktorekdev/gburn
bun add -g @wiktorekdev/gburn
```

## Usage

```bash
gburn                 # fullscreen TUI (after global install)
gburn --summary       # text table
gburn --json          # machine-readable
gburn --cwd ideas     # filter by project path
gburn --home D:\grok  # custom GROK_HOME
```

### Keys

| Key | Action |
|-----|--------|
| `↑` `↓` / `j` `k` | Move |
| `Enter` | Session detail |
| `/` | Search |
| `s` | Cycle sort |
| `p` | Pricing table |
| `m` | Usage by model |
| `r` | Rescan |
| `q` | Quit |

## How it works

```
~/.grok/sessions/<project>/<session-id>/
  summary.json
  updates.jsonl      → token streams
  chat_history.jsonl → real model ids
  signals.json
  subagents/*/meta.json
```

- **input** ≈ `totalTokens` at each model stream start  
- **output** ≈ growth on agent events  
- **model** from assistant messages in `chat_history` (not flaky session primary)  
- **multi-model** sessions split by message share  
- **subagents** linked parent ↔ child  

Empty sessions are hidden.

> Hypothetical list-price cost — not SuperGrok / Cursor subscription billing.

## Dev

```bash
git clone https://github.com/wiktorekdev/gburn.git
cd gburn
bun start
bun run build
```

Requires [Bun](https://bun.sh) or Node 18+ for the published CLI.

## License

[MIT](./LICENSE) © wiktorekdev
