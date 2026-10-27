//! The fail-first check: a task is fair only if its hidden tests fail
//! where the agent starts and pass on a correct answer.
//!
//! Run it before a task joins a suite. It also warms the task's cargo
//! caches, so the agent's first build doesn't start cold.

use std::path::Path;

use anyhow::Result;

use crate::check::{self, CheckOutcome};
use crate::project::{self, Stage};
use crate::task::Task;
use crate::util;

/// Check every task; true when all of them are fair.
pub fn verify(tasks: &[&Task], koda_repo: &Path) -> Result<bool> {
    let mut all_fair = true;
    for task in tasks {
        let work = util::cache_dir().join("verify").join(task.id());
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work)?;
        let target = util::cache_dir().join("target").join(task.id());

        eprint!("{} ... ", task.id());
        let start = check_stage(task, Stage::Start, koda_repo, &work, &target)?;
        let solved = check_stage(task, Stage::Solved, koda_repo, &work, &target)?;

        if !start.passed() && solved.passed() {
            eprintln!("fair: start {}, solution passes", start.failure_kind());
        } else {
            all_fair = false;
            let start = if start.passed() {
                "start already passes (the task is free)"
            } else {
                start.failure_kind()
            };
            let solved = if solved.passed() {
                "passes"
            } else {
                solved.failure_kind()
            };
            eprintln!(
                "NOT FAIR: start {start}, solution {solved}. Logs: {}",
                work.display()
            );
        }
    }
    Ok(all_fair)
}

fn check_stage(
    task: &Task,
    stage: Stage,
    koda_repo: &Path,
    work: &Path,
    target: &Path,
) -> Result<CheckOutcome> {
    let name = match stage {
        Stage::Start => "start",
        Stage::Solved => "solved",
    };
    let project = work.join(name);
    project::materialize(task, stage, koda_repo, &project)?;
    project::inject_hidden(task, &project)?;
    check::run(task, &project, target, &work.join(format!("{name}.log")))
}
