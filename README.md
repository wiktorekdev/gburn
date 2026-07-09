# gburn

<p align="center">
  <strong>Grok Build burn</strong>: session tokens and public API list-price cost, fullscreen TUI.
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

### Keys and mouse

| Input | Action |
|-------|--------|
| `↑` `↓` / `j` `k` | Move |
| click | Select row |
| double-click / `Enter` | Open detail |
| scroll wheel | Move / scroll detail |
| `/` | Search |
| `s` | Sort |
| `p` | Prices |
| `m` | By model |
| `r` | Rescan |
| `q` | Quit |

## Pricing

Costs are **public API list rates** (xAI / Cursor), not subscription invoices.

| Model | $/1M in | $/1M out |
|-------|---------|----------|
| Grok 4.5 | $2 | $6 |
| Composer 2.5 Fast | $3 | $15 |
| grok-build | $1 | $2 |

AI labs often **subsidize inference** (sell below true compute cost) to win users and lock in ecosystems. `gburn` shows what the same token volume would cost at **list price**, so you can see the scale of that burn even when your plan hides it behind a flat fee.

Data source: local `~/.grok/sessions`.

## Dev

```bash
git clone https://github.com/wiktorekdev/gburn.git
cd gburn
bun start
bun run build
```

TUI: custom TypeScript + ANSI (no React/Ink).

## License

[MIT](./LICENSE) © wiktorekdev
