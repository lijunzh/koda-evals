//! The fail-first check: a task is fair only if its grading can tell a
//! solution from the starting point.
//!
//! Hidden tests must fail where the agent starts and pass on a correct
//! answer. Transcript predicates can't be run against a model here, so
//! they are checked against two canned transcripts the task carries:
//! `hidden/should-pass.ndjson` must pass every predicate and
//! `hidden/should-fail.ndjson` must fail at least one. A predicate set
//! that passes both, or fails both, is the transcript version of a free
//! or impossible task.
//!
//! Run it before a task joins a suite. It also warms the task's cargo
//! caches, so the agent's first build doesn't start cold.

use std::path::Path;

use anyhow::{Context, Result};

use crate::check::{self, HiddenOutcome, TranscriptOutcome};
use crate::project::{self, Stage};
use crate::task::Task;
use crate::transcript::Transcript;
use crate::util;

/// Check every task; true when all of them are fair.
pub fn verify(tasks: &[&Task], koda_repo: &Path) -> Result<bool> {
    let mut all_fair = true;
    for task in tasks {
        eprint!("{} ... ", task.id());
        let mut fair = true;
        let mut notes = Vec::new();

        if task.has_hidden() {
            let (ok, note) = verify_hidden(task, koda_repo)?;
            fair &= ok;
            notes.push(note);
        }
        if task.has_predicates() {
            let (ok, note) = verify_predicates(task)?;
            fair &= ok;
            notes.push(note);
        }

        all_fair &= fair;
        let verdict = if fair { "fair" } else { "NOT FAIR" };
        eprintln!("{verdict}: {}", notes.join("; "));
    }
    Ok(all_fair)
}

/// Hidden tests: fail at the start, pass on the solution.
fn verify_hidden(task: &Task, koda_repo: &Path) -> Result<(bool, String)> {
    let work = util::cache_dir().join("verify").join(task.id());
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    let target = util::cache_dir().join("target").join(task.id());

    let stages = check_stage(task, Stage::Start, koda_repo, &work, &target).and_then(|start| {
        let solved = check_stage(task, Stage::Solved, koda_repo, &work, &target)?;
        Ok((start, solved))
    });
    // Agents can read this cache, and the stages hold the solution and the
    // hidden tests. Only the logs of an unfair task stay.
    for stage in ["start", "solved"] {
        let _ = std::fs::remove_dir_all(work.join(stage));
    }
    let (start, solved) = stages?;

    if !start.passed() && solved.passed() {
        let _ = std::fs::remove_dir_all(&work);
        return Ok((
            true,
            format!("start {}, solution passes", start.failure_kind()),
        ));
    }
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
    Ok((
        false,
        format!("start {start}, solution {solved}. Logs: {}", work.display()),
    ))
}

/// Predicates: pass the should-pass transcript, fail the should-fail one.
fn verify_predicates(task: &Task) -> Result<(bool, String)> {
    let predicates = &task.spec.check.transcript;
    let read = |path: &Path| -> Result<TranscriptOutcome> {
        let t = Transcript::read(path).with_context(|| format!("read {}", path.display()))?;
        Ok(check::evaluate_predicates(predicates, &t, None))
    };
    let should_pass = read(&task.canned_pass())?;
    let should_fail = read(&task.canned_fail())?;
    Ok(predicate_verdict(&should_pass, &should_fail))
}

fn predicate_verdict(
    should_pass: &TranscriptOutcome,
    should_fail: &TranscriptOutcome,
) -> (bool, String) {
    let n = should_pass.predicates.len();
    match (should_pass.first_failure(), should_fail.first_failure()) {
        (None, Some(v)) => (
            true,
            format!(
                "{n} predicate(s): should-pass passes, should-fail fails {} ({})",
                v.predicate, v.detail
            ),
        ),
        (Some(v), Some(_)) => (
            false,
            format!(
                "should-pass fails {} ({}); the predicates or the transcript are wrong",
                v.predicate, v.detail
            ),
        ),
        (None, None) => (
            false,
            format!(
                "should-fail passes all {n} predicate(s); the set can't fail (the task is free)"
            ),
        ),
        (Some(a), None) => (
            false,
            format!(
                "should-pass fails {} ({}) and should-fail passes; the transcripts are \
                 swapped or the predicates are inverted",
                a.predicate, a.detail
            ),
        ),
    }
}

fn check_stage(
    task: &Task,
    stage: Stage,
    koda_repo: &Path,
    work: &Path,
    target: &Path,
) -> Result<HiddenOutcome> {
    let name = match stage {
        Stage::Start => "start",
        Stage::Solved => "solved",
    };
    let project = work.join(name);
    project::materialize(task, stage, koda_repo, &project)?;
    project::inject_hidden(task, &project)?;
    check::run(task, &project, target, &work.join(format!("{name}.log")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Verdict;

    fn outcome(fails: &[&str]) -> TranscriptOutcome {
        let predicates = ["no_edit", "turn_end complete"]
            .iter()
            .map(|p| Verdict {
                predicate: p.to_string(),
                pass: !fails.contains(p),
                detail: format!("detail of {p}"),
            })
            .collect::<Vec<_>>();
        TranscriptOutcome {
            pass: predicates.iter().all(|v| v.pass),
            predicates,
        }
    }

    #[test]
    fn a_set_is_fair_only_when_it_splits_the_two_transcripts() {
        let (fair, note) = predicate_verdict(&outcome(&[]), &outcome(&["no_edit"]));
        assert!(fair);
        assert!(
            note.contains("should-fail fails no_edit (detail of no_edit)"),
            "{note}"
        );

        let (fair, note) = predicate_verdict(&outcome(&[]), &outcome(&[]));
        assert!(!fair);
        assert!(note.contains("can't fail"), "{note}");

        let (fair, note) =
            predicate_verdict(&outcome(&["turn_end complete"]), &outcome(&["no_edit"]));
        assert!(!fair);
        assert!(
            note.contains("should-pass fails turn_end complete"),
            "{note}"
        );

        let (fair, _) = predicate_verdict(&outcome(&["no_edit"]), &outcome(&[]));
        assert!(!fair);
    }
}
