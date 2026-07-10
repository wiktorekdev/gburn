mod demo;
mod estimator;
mod format;
mod pricing;
mod report;
mod scanner;
mod tui;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;

use demo::{debug_scan, demo_scan};
use report::{print_csv, print_json, print_summary};
use scanner::{scan_sessions, ScanResult};

#[derive(Parser, Debug)]
#[command(
    name = "gburn",
    version,
    about = "Grok Build token burn at public API list prices",
    long_about = "Fullscreen TUI for local Grok Build session token usage and list-price cost.\nReads ~/.grok/sessions."
)]
struct Cli {
    /// Text summary (no TUI)
    #[arg(short = 's', long = "summary")]
    summary: bool,

    /// JSON report
    #[arg(short = 'j', long = "json")]
    json: bool,

    /// CSV export
    #[arg(long = "csv")]
    csv: bool,

    /// Filter by project path substring
    #[arg(long = "cwd", value_name = "PATH")]
    cwd: Option<String>,

    /// Override GROK_HOME (default: ~/.grok)
    #[arg(long = "home", value_name = "PATH", env = "GROK_HOME")]
    home: Option<PathBuf>,

    /// Sample sessions for screenshots / demos
    #[arg(long = "demo", env = "GBURN_DEMO")]
    demo: bool,

    /// Fake data: total USD burn + session count (grok-4.5 only)
    /// Example: --debug 13000 10
    #[arg(long = "debug", num_args = 2, value_names = ["USD", "SESSIONS"])]
    debug: Option<Vec<String>>,
}

enum Source {
    Live {
        home: Option<PathBuf>,
        cwd: Option<String>,
    },
    Demo,
    Debug {
        usd: f64,
        n: usize,
    },
}

impl Source {
    fn scan(&self) -> ScanResult {
        match self {
            Source::Live { home, cwd } => scan_sessions(home.clone(), cwd.as_deref()),
            Source::Demo => demo_scan(),
            Source::Debug { usd, n } => debug_scan(*usd, *n),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let source = if let Some(args) = &cli.debug {
        let usd: f64 = args[0]
            .parse()
            .map_err(|_| anyhow::anyhow!("--debug USD must be a number, got {}", args[0]))?;
        let n: usize = args[1]
            .parse()
            .map_err(|_| anyhow::anyhow!("--debug SESSIONS must be an integer, got {}", args[1]))?;
        if n == 0 {
            anyhow::bail!("--debug SESSIONS must be >= 1");
        }
        Source::Debug { usd, n }
    } else if cli.demo
        || matches!(
            std::env::var("GBURN_DEMO").as_deref(),
            Ok("1") | Ok("true")
        )
    {
        Source::Demo
    } else {
        Source::Live {
            home: cli.home.clone(),
            cwd: cli.cwd.clone(),
        }
    };

    let is_fake = matches!(source, Source::Demo | Source::Debug { .. });

    let interactive = !cli.json
        && !cli.summary
        && !cli.csv
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal();

    if !cli.json && !cli.csv && !interactive && !is_fake {
        eprintln!("Scanning Grok Build sessions…");
    }

    let started = Instant::now();
    let scan = source.scan();
    let elapsed = started.elapsed();

    if cli.json {
        print_json(&scan);
        return Ok(());
    }

    if cli.csv {
        print_csv(&scan);
        return Ok(());
    }

    if !interactive {
        if !is_fake {
            eprintln!(
                "Found {} sessions in {:.0}ms",
                scan.sessions.len(),
                elapsed.as_secs_f64() * 1000.0
            );
        }
        print_summary(&scan);
        return Ok(());
    }

    if scan.sessions.is_empty() && !is_fake {
        eprintln!(
            "No priced sessions found in {}",
            scan.sessions_dir.display()
        );
        eprintln!("Tip: gburn --demo   or   gburn --debug 13000 10");
        print_summary(&scan);
        return Ok(());
    }

    let data = match source {
        Source::Live { home, cwd } => tui::DataSource::Live { home, cwd },
        Source::Demo => tui::DataSource::Demo,
        Source::Debug { usd, n } => tui::DataSource::Debug { usd, n },
    };
    tui::run_tui(scan, data)
}
