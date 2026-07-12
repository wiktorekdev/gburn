# gburn

<p align="center">
  <strong>Grok Build burn</strong>: session tokens and public API list-price cost, fullscreen TUI.
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@wiktorekdev/gburn"><img src="https://img.shields.io/npm/v/@wiktorekdev/gburn.svg?style=flat-square&color=cb3837" alt="npm version" /></a>
  <a href="https://www.npmjs.com/package/@wiktorekdev/gburn"><img src="https://img.shields.io/npm/dm/@wiktorekdev/gburn.svg?style=flat-square&color=blue" alt="npm downloads" /></a>
  <a href="https://github.com/wiktorekdev/gburn/blob/main/LICENSE"><img src="https://img.shields.io/npm/l/@wiktorekdev/gburn.svg?style=flat-square" alt="license" /></a>
  <a href="https://nodejs.org"><img src="https://img.shields.io/node/v/@wiktorekdev/gburn.svg?style=flat-square&color=brightgreen" alt="node" /></a>
  <a href="https://github.com/wiktorekdev/gburn/releases/latest"><img src="https://img.shields.io/github/v/release/wiktorekdev/gburn?style=flat-square&label=release" alt="release" /></a>
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

# or cargo / from source
cargo install --git https://github.com/wiktorekdev/gburn --tag v2.0.0
cargo install --path .
```

Binaries also on [Releases](https://github.com/wiktorekdev/gburn/releases).

## Usage

```bash
gburn                      # TUI
gburn --summary
gburn --json
gburn --csv
gburn --cwd ideas
gburn --home ~/.grok
gburn --demo               # sample data
gburn --debug 13000 10     # fake $13k / 10 grok-4.5 sessions
```

### Keys and mouse

| Input | Action |
|-------|--------|
| `↑` `↓` / `j` `k` | Move |
| click | Select row / tab / chip |
| double-click / `Enter` | Open detail |
| right-click / `Esc` | Back |
| scroll wheel | Move / scroll |
| `/` | Search |
| `s` | Sort |
| `t` | Time range |
| `a` | Toggle subagents |
| `1`–`6` / `Tab` | Pages |
| `r` | Rescan |
| `q` | Quit |

### Pages

- **overall** — burn intro (rocket / casino / inferno) + totals
- **sessions** — list + session detail
- **projects** — by cwd + project detail
- **models** — by model + model detail
- **pricing** — list rates
- **help** — shortcuts

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
cargo run
cargo run -- --debug 13000 10
cargo build --release
```

TUI: **Rust + ratatui + crossterm**.

### Release

1. Bump `version` in `Cargo.toml` + `package.json`
2. Push to `main`
3. Create a GitHub Release (`v2.0.0` etc.)

CI builds platform binaries, attaches them to the release, then publishes `@wiktorekdev/gburn` to npm (OIDC trusted publish). The npm package is a thin installer: `postinstall` fetches the matching binary.

## License

[MIT](./LICENSE) © wiktorekdev
