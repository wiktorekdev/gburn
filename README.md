# gburn

<p align="center">
  <strong>Grok Build burn rate</strong> — session tokens & API list-price cost, in a fullscreen TUI.
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

## Install

```bash
npx @wiktorekdev/gburn
bunx @wiktorekdev/gburn

npm i -g @wiktorekdev/gburn   # → gburn
```

## Usage

```bash
gburn                 # TUI
gburn --summary       # text table
gburn --json          # JSON
gburn --cwd ideas     # filter project
gburn --home ~/.grok  # custom GROK_HOME
```

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

## Pricing

| Model | $/1M in | $/1M out |
|-------|---------|----------|
| Grok 4.5 | $2 | $6 |
| Composer 2.5 Fast | $3 | $15 |
| grok-build | $1 | $2 |

Reads `~/.grok/sessions` (local only).

## Dev

```bash
git clone https://github.com/wiktorekdev/gburn.git
cd gburn
bun start
bun run build
```

## License

[MIT](./LICENSE) © wiktorekdev
