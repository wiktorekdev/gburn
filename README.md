# gburn

<p align="center">
  <strong>Grok Build burn</strong>: session tokens, API list price, and firm subsidy.
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@wiktorekdev/gburn"><img src="https://img.shields.io/npm/v/@wiktorekdev/gburn.svg?style=flat-square&color=cb3837" alt="npm version" /></a>
  <a href="https://www.npmjs.com/package/@wiktorekdev/gburn"><img src="https://img.shields.io/npm/dm/@wiktorekdev/gburn.svg?style=flat-square&color=blue" alt="npm downloads" /></a>
  <a href="https://github.com/wiktorekdev/gburn/blob/main/LICENSE"><img src="https://img.shields.io/npm/l/@wiktorekdev/gburn.svg?style=flat-square" alt="license" /></a>
  <a href="https://nodejs.org"><img src="https://img.shields.io/node/v/@wiktorekdev/gburn.svg?style=flat-square&color=brightgreen" alt="node" /></a>
  <a href="https://github.com/wiktorekdev/gburn"><img src="https://img.shields.io/github/stars/wiktorekdev/gburn?style=flat-square" alt="stars" /></a>
</p>

<p align="center">
  <code>npx @wiktorekdev/gburn</code> · <code>bunx @wiktorekdev/gburn</code>
</p>

## Install

```bash
npx @wiktorekdev/gburn
bunx @wiktorekdev/gburn
npm i -g @wiktorekdev/gburn
```

## Usage

```bash
gburn                 # TUI
gburn --summary
gburn --json
gburn --cwd ideas
gburn --home ~/.grok
```

| Env | Meaning |
|-----|---------|
| `GBURN_BILLING=sub` | default: LIST = what API would charge, SUB = firm subsidy, YOU = $0 |
| `GBURN_BILLING=api` | LIST = what you pay, SUB = $0 |
| `GBURN_DEMO=1` | sample sessions for screenshots |

### Keys

| Key | Action |
|-----|--------|
| `↑` `↓` / `j` `k` | Move |
| `Enter` | Detail |
| `/` | Search |
| `s` | Sort |
| `p` | Prices |
| `m` | By model |
| `r` | Rescan |
| `q` | Quit |

## Models (list $/1M)

| Model | In | Out |
|-------|----|-----|
| Grok 4.5 | $2 | $6 |
| Composer 2.5 Fast | $3 | $15 |
| grok-build | $1 | $2 |

Data: local `~/.grok/sessions`.

## Dev

```bash
git clone https://github.com/wiktorekdev/gburn.git
cd gburn
bun start
bun run build
```

TUI is custom TypeScript + ANSI (no React/Ink). Fast redraw, zero runtime deps.

## License

[MIT](./LICENSE) © wiktorekdev
