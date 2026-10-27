//! `koda-eval`: runs koda binaries on fixed coding tasks and records
//! whether each task's hidden tests pass, with tokens and time.
//!
//! It reads koda only through `koda -p … --output-format stream-json`, so
//! any two koda builds can be compared without linking koda's crates.

mod build;
mod check;
mod keys;
mod proc;
mod project;
mod report;
mod run;
mod task;
mod util;
mod verify;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "koda-eval", version, about)]
struct Cli {
    /// The eval repo: holds tasks/, runs/ and results/.
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    /// Koda checkout that history tasks are unpacked from.
    #[arg(long, global = true, env = "KODA_REPO", default_value = "~/repo/koda")]
    koda_repo: String,

    /// Koda keystore to copy API keys from, for keys not already in the
    /// environment. Default: ~/.config/koda/db/koda.db.
    #[arg(long, global = true, value_name = "PATH")]
    keystore: Option<PathBuf>,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List the tasks and their tags.
    List,
    /// Fail-first check: hidden tests must fail where the agent starts and
    /// pass on the task's solution.
    Verify {
        /// Task ids (default: every task).
        tasks: Vec<String>,
    },
    /// Run every task × variant × model × repeat in a run file.
    Run {
        run_file: PathBuf,
        /// Keep each run's scratch project under ~/.cache/koda-eval/work.
        #[arg(long)]
        keep_work: bool,
    },
    /// Summarize a results directory: passes, pass^k, tokens, time, and the
    /// tasks where variants disagree. Also written there as report.md.
    Report {
        /// A results directory, or a run name (its latest results).
        what: PathBuf,
    },
}

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn real_main() -> Result<ExitCode> {
    let cli = Cli::parse();
    let koda_repo = util::expand(&cli.koda_repo);
    let tasks_dir = cli.root.join("tasks");
    match cli.command {
        Cmd::List => {
            for t in task::load_all(&tasks_dir)? {
                println!("{:<28} {}", t.id(), t.spec.tags.join(", "));
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Verify { tasks } => {
            let all = task::load_all(&tasks_dir)?;
            let selected = task::select(&all, &tasks, &[])?;
            let fair = verify::verify(&selected, &koda_repo)?;
            Ok(if fair {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Cmd::Run {
            run_file,
            keep_work,
        } => {
            let opts = run::Options {
                root: cli.root,
                koda_repo,
                keystore: cli.keystore.unwrap_or_else(keys::default_keystore),
                keep_work,
            };
            let out = run::run(&run_file, &opts)?;
            println!("results: {}", out.display());
            report::report(&out)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Report { what } => {
            let dir = report::locate(&cli.root, &what)?;
            report::report(&dir)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
