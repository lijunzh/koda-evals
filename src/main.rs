//! `koda-eval`: runs koda binaries on fixed coding tasks and records
//! whether each task's hidden tests pass, with tokens and time.
//!
//! It reads koda only through `koda -p … --output-format stream-json`, so
//! any two koda builds can be compared without linking koda's crates.

mod build;
mod check;
mod keys;
mod klog;
mod proc;
mod project;
mod regrade;
mod report;
mod run;
mod task;
mod transcript;
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
    /// List the tasks with their cluster, grade and tags.
    List,
    /// Fail-first check: hidden tests must fail where the agent starts and
    /// pass on the task's solution; transcript predicates must pass the
    /// task's should-pass transcript and fail its should-fail one.
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
        /// Runs to execute at once, over the run file's `jobs`; 1 if neither
        /// says. 3 or 4 suits an API model, bounded by the cargo builds of
        /// the hidden tests, not by the API. A local model shares one GPU
        /// and, by default, one context cache across its slots, so measure
        /// before going above 1 (see `bin/lms-parallel-probe`). Wall times
        /// in the report are contended above 1.
        #[arg(long, value_parser = clap::value_parser!(u16).range(1..))]
        jobs: Option<u16>,
    },
    /// Summarize a results directory: passes, pass^k, tokens, time, and the
    /// tasks where variants disagree. Also written there as report.md.
    Report {
        /// A results directory, or a run name (its latest results).
        what: PathBuf,
    },
    /// Grade a results directory again from its saved records, against
    /// the tasks' current checks, and rewrite its report. The stored hidden
    /// test outcomes stand. For a grading fix that changes no run.
    Regrade {
        /// A results directory, or a run name (its latest results).
        what: PathBuf,
    },
    /// Write each run's `record.json` from its `transcript.ndjson`: once for
    /// a results directory saved before records existed, or with `--force`
    /// after re-translating a wrapper's transcript.
    Record {
        /// Results directories, or run names (their latest results).
        what: Vec<PathBuf>,
        /// Rewrite records that already exist.
        #[arg(long)]
        force: bool,
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
                println!(
                    "{:<28} {:<14} {:<6} {}",
                    t.id(),
                    t.spec.cluster,
                    t.spec.grade,
                    t.spec.tags.join(", ")
                );
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
            jobs,
        } => {
            let opts = run::Options {
                root: cli.root,
                koda_repo,
                keystore: cli.keystore.unwrap_or_else(keys::default_keystore),
                keep_work,
                jobs: jobs.map(usize::from),
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
        Cmd::Record { what, force } => {
            for what in &what {
                let dir = report::locate(&cli.root, what)?;
                let written = regrade::record(&dir, force)?;
                println!("{}: {written} record(s) written", dir.display());
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Regrade { what } => {
            let dir = report::locate(&cli.root, &what)?;
            let changed = regrade::regrade(&dir, &task::load_all(&tasks_dir)?)?;
            println!("{changed} row(s) changed verdict");
            report::report(&dir)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
