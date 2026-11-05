//! `koda-eval regrade`: a saved run graded again from its transcript, with
//! its stored hidden-test outcome. No agent and no model.

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

const TASK: &str = "behaviour-answer-vs-act-control";

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), to).unwrap();
        }
    }
}

/// A results directory with one row for `TASK`, as a run left it: hidden
/// tests passed, the transcript graded as failing.
fn results_dir(root: &Path, transcript: &str) -> std::path::PathBuf {
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tasks")
            .join(TASK),
        &root.join("tasks").join(TASK),
    );

    let dir = root.join("results/demo/20260101T000000Z");
    let art = dir.join(TASK).join("codex.m.r1");
    std::fs::create_dir_all(&art).unwrap();
    std::fs::write(art.join("transcript.ndjson"), transcript).unwrap();
    let row = json!({
        "task": TASK, "variant": "codex", "model": "m", "repeat": 1,
        "artifacts": format!("{TASK}/codex.m.r1"),
        "pass": false, "failure": "transcript: tool_count Edit min=1",
        "agent": {"exit": 0, "timed_out": false, "wall_ms": 1000},
        "behaviour": {"from_koda_log": 7},
        "diff": {"files": 1, "insertions": 1, "deletions": 1},
        "check": {
            "hidden": {"exit": 0, "timed_out": false, "hidden_ok": 2, "hidden_failed": 0,
                       "compile_error": false, "tail": ""},
            "transcript": null
        }
    });
    std::fs::write(dir.join("results.jsonl"), format!("{row}\n")).unwrap();
    std::fs::write(dir.join("meta.json"), json!({"tasks": [TASK]}).to_string()).unwrap();
    dir
}

fn transcript(edit_tool: &str) -> String {
    [
        json!({"type": "tool_call_start", "id": "1", "name": edit_tool,
               "args": {"command": "apply_patch < /tmp/fix.patch", "file_path": "src/lib.rs"}}),
        json!({"type": "tool_call_result", "id": "1", "name": edit_tool, "output": "ok"}),
        json!({"type": "turn_end", "reason": {"kind": "complete"}}),
    ]
    .iter()
    .map(|e| format!("{e}\n"))
    .collect()
}

fn regrade(root: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_koda-eval"))
        .arg("--root")
        .arg(root)
        .args(["regrade", "demo"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn row(dir: &Path) -> Value {
    let text = std::fs::read_to_string(dir.join("results.jsonl")).unwrap();
    serde_json::from_str(text.lines().next().unwrap()).unwrap()
}

#[test]
fn an_unchanged_transcript_keeps_its_verdict() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = results_dir(tmp.path(), &transcript("Bash"));
    assert!(regrade(tmp.path()).starts_with("0 row(s) changed verdict"));
    let r = row(&dir);
    assert_eq!(r["pass"], false);
    assert!(r.get("regraded").is_none(), "{r:#}");
}

#[test]
fn a_fixed_transcript_changes_the_verdict_and_keeps_the_old_one() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = results_dir(tmp.path(), &transcript("Edit"));
    assert!(regrade(tmp.path()).starts_with("1 row(s) changed verdict"));
    let r = row(&dir);
    assert_eq!(r["pass"], true, "{r:#}");
    assert_eq!(r["failure"], Value::Null);
    assert_eq!(r["regraded"]["pass"], false);
    assert_eq!(
        r["regraded"]["failure"],
        "transcript: tool_count Edit min=1"
    );
    // The hidden outcome stands; what came from koda's log stays.
    assert_eq!(r["check"]["hidden"]["hidden_ok"], 2);
    assert_eq!(r["behaviour"]["from_koda_log"], 7);
    assert_eq!(r["events"]["tool_call_start"], 1);
    assert!(dir.join("report.md").is_file());

    // Again: nothing changes, and the first verdict is still the one kept.
    assert!(regrade(tmp.path()).starts_with("0 row(s) changed verdict"));
    assert_eq!(row(&dir)["regraded"]["pass"], false);
}
