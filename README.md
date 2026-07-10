# gburn

**Grok Build burn** — session tokens and public API list-price cost. Native Rust TUI.

## Install

```bash
# npm (downloads platform binary from GitHub Releases)
npm i -g @wiktorekdev/gburn

# cargo
cargo install --git https://github.com/wiktorekdev/gburn --tag v2.0.0

# from source
cargo install --path .
```

Or grab a binary from [Releases](https://github.com/wiktorekdev/gburn/releases).

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

### Keys

| Key | Action |
|-----|--------|
| `j` `k` / arrows | Move |
| `Enter` | Open detail |
| `Esc` | Back |
| `/` | Search |
| `s` | Sort |
| `t` | Time range |
| `a` | Toggle subagents |
| `1`–`6` / `Tab` | Pages |
| `r` | Rescan |
| `q` | Quit |

Click tabs, chips, rows; double-click opens; wheel scrolls; right-click back.

### Pages

- **overall** — burn intro (rocket / casino / inferno) + totals
- **sessions** — list + session detail
- **projects** — by cwd + project detail
- **models** — by model + model detail
- **pricing** — list rates
- **help** — shortcuts

## Pricing

Public API list rates (not invoices).

| Model | $/1M in | $/1M out |
|-------|---------|----------|
| Grok 4.5 | $2 | $6 |
| Composer 2.5 Fast | $3 | $15 |
| grok-build | $1 | $2 |

Data: local `~/.grok/sessions`.

## Dev

```bash
cargo run
cargo run -- --debug 13000 10
cargo build --release
```

## License

MIT © wiktorekdev
