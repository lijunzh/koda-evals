//! Grade a results directory again from its saved transcripts.
//!
//! For a grading fix that changes no run: a transcript predicate, or a
//! wrapper's translation of an agent's events (re-translate its saved
//! stream into `transcript.ndjson` first). The hidden tests aren't rerun;
//! their stored outcome stands, since the project they ran in is gone. A row
//! whose verdict changes keeps the old one under `regraded`.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::check::{self, HiddenOutcome};
use crate::project::DiffStat;
use crate::task::Task;
use crate::transcript::Transcript;

/// Regrade every row of `dir/results.jsonl` against `tasks`, rewriting the
/// file. Returns how many rows changed verdict.
pub fn regrade(dir: &Path, tasks: &[Task]) -> Result<usize> {
    let path = dir.join("results.jsonl");
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    let mut out = String::new();
    let mut changed = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut row: Value = serde_json::from_str(line)?;
        if regrade_row(dir, tasks, &mut row)? {
            changed += 1;
        }
        out.push_str(&serde_json::to_string(&row)?);
        out.push('\n');
    }
    let tmp = dir.join("results.jsonl.regrade");
    std::fs::write(&tmp, out)?;
    std::fs::rename(&tmp, &path)?;
    Ok(changed)
}

/// Regrade one row in place. Rows without a transcript (a harness error) or
/// a task that no longer exists are left alone.
fn regrade_row(dir: &Path, tasks: &[Task], row: &mut Value) -> Result<bool> {
    let (Some(task_id), Some(art)) = (row["task"].as_str(), row["artifacts"].as_str()) else {
        return Ok(false);
    };
    let Some(task) = tasks.iter().find(|t| t.id() == task_id) else {
        return Ok(false);
    };
    let transcript_path = dir.join(art).join("transcript.ndjson");
    if row["pass"].is_null() || !transcript_path.exists() {
        return Ok(false);
    }
    let transcript = Transcript::read(&transcript_path)?;
    let hidden: Option<HiddenOutcome> = serde_json::from_value(row["check"]["hidden"].clone())?;
    let diff: Option<DiffStat> = serde_json::from_value(row["diff"].clone()).ok();
    let check = check::evaluate(task, hidden, &transcript, diff.as_ref());
    let exit = row["agent"]["exit"].as_i64().map(|e| e as i32);
    let timed_out = row["agent"]["timed_out"].as_bool().unwrap_or(false);
    let (pass, failure) = crate::run::verdict(&check, &transcript, exit, timed_out);

    let was = (row["pass"].clone(), row["failure"].clone());
    let flipped = was.0 != json!(pass) || was.1 != json!(failure);
    if flipped && row.get("regraded").is_none() {
        row["regraded"] = json!({ "pass": was.0, "failure": was.1 });
    }
    row["pass"] = json!(pass);
    row["failure"] = json!(failure);
    row["check"] = serde_json::to_value(&check)?;
    row["events"] = serde_json::to_value(&transcript.counts)?;
    if let Some(summary) = &transcript.summary {
        row["summary"] = summary.clone();
    }
    // The transcript's part of `behaviour`; what came from koda's log stays.
    if let (Some(row_b), Value::Object(fresh)) = (
        row["behaviour"].as_object_mut(),
        transcript.behaviour(timed_out),
    ) {
        row_b.extend(fresh);
    }
    Ok(flipped)
}
