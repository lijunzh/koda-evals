//! The wire protocol through the runner, verdict included: `koda-eval run`
//! against a fake `koda` that prints a transcript. One is a capture of the real
//! binary (`bin/capture-koda-wire`). No model and no API key.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

const CAPTURED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/koda-wire-v1.ndjson"
);

/// Graded by the transcript alone, so nothing but the stream decides it.
const TASK: &str = r#"id = "wire"
prompt = "Fix the parser."
[repo]
fixture = "fixture"
solution = "fixture"
[check]
[[check.transcript]]
type = "turn_end"
kind = "complete"
"#;

const RUN: &str = r#"name = "wire"
tasks = ["wire"]
[[variants]]
name = "koda"
bin = "BIN"
[[models]]
name = "stub"
provider = "stub"
"#;

struct Finished {
    row: Value,
    stdout: String,
    stderr: String,
}

/// `koda-eval run` on one task, with a `koda` that prints `stream`.
fn score(stream: &Path) -> Finished {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let fixture = root.join("tasks/wire/fixture");
    std::fs::create_dir_all(&fixture).unwrap();
    std::fs::write(fixture.join("README.md"), "A project.\n").unwrap();
    std::fs::write(root.join("tasks/wire/task.toml"), TASK).unwrap();
    // `verify` reads these; loading a task only asks that they exist.
    std::fs::create_dir_all(root.join("tasks/wire/hidden")).unwrap();
    for canned in ["should-pass", "should-fail"] {
        let path = root.join(format!("tasks/wire/hidden/{canned}.ndjson"));
        std::fs::write(path, "").unwrap();
    }

    let koda = root.join("koda");
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo 'koda 9.9.9'; exit 0; fi\n\
         cat '{}'\n",
        stream.display()
    );
    std::fs::write(&koda, script).unwrap();
    std::fs::set_permissions(&koda, std::fs::Permissions::from_mode(0o755)).unwrap();

    let run_file = root.join("wire.toml");
    let run = RUN.replace("BIN", koda.to_str().unwrap());
    std::fs::write(&run_file, run).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_koda-eval"))
        .arg("--root")
        .arg(root)
        .arg("run")
        .arg(&run_file)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "stdout:\n{stdout}\nstderr:\n{stderr}");

    let runs: Vec<_> = std::fs::read_dir(root.join("results/wire"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(runs.len(), 1);
    let rows = std::fs::read_to_string(runs[0].join("results.jsonl")).unwrap();
    let row = serde_json::from_str(rows.lines().next().unwrap()).unwrap();
    Finished {
        row,
        stdout,
        stderr,
    }
}

/// A finished turn that opens with this `hello`.
fn score_hello(hello: &str) -> Finished {
    let tmp = tempfile::tempdir().unwrap();
    let stream = tmp.path().join("stream.ndjson");
    let turn = r#"{"type":"turn_start","session_id":"s","turn_id":"t"}
{"type":"turn_end","session_id":"s","turn_id":"t","reason":{"kind":"complete"}}
{"type":"run_summary","session_id":"s","success":true,"response":"Done.","turn_end":{"kind":"complete"}}
"#;
    std::fs::write(&stream, format!("{hello}\n{turn}")).unwrap();
    score(&stream)
}

#[test]
fn a_supported_protocol_keeps_its_pass() {
    let run = score_hello(r#"{"type":"hello","protocol_version":1}"#);
    assert_eq!(run.row["pass"], true, "{:#}", run.row);
    assert_eq!(run.row["failure"], Value::Null, "{:#}", run.row);
    assert!(run.stdout.contains("| 1/1 | 100% |"), "{}", run.stdout);
}

#[test]
fn an_unsupported_protocol_fails_when_every_check_passes() {
    let run = score_hello(r#"{"type":"hello","protocol_version":2}"#);
    let row = &run.row;
    // The checks did pass; that's the case the verdict has to overrule.
    assert_eq!(row["check"]["transcript"]["pass"], true, "{row:#}");
    assert_eq!(row["agent"]["exit"], 0, "{row:#}");
    assert_eq!(row["pass"], false, "{row:#}");
    assert_eq!(row["failure"], "unsupported protocol 2", "{row:#}");
    assert_eq!(row["behaviour"]["protocol_version"], 2, "{row:#}");
    assert!(
        run.stderr.contains("FAIL (unsupported protocol 2)"),
        "{}",
        run.stderr
    );
    assert!(run.stdout.contains("| 0/1 | 0% |"), "{}", run.stdout);
}

#[test]
fn a_hello_with_no_readable_version_fails_too() {
    let run = score_hello(r#"{"type":"hello"}"#);
    assert_eq!(run.row["pass"], false, "{:#}", run.row);
    assert_eq!(run.row["failure"], "unsupported protocol", "{:#}", run.row);
}

#[test]
fn a_run_from_before_the_protocol_keeps_its_pass() {
    let run = score_hello(r#"{"type":"turn_start","turn_id":"0"}"#);
    assert_eq!(run.row["pass"], true, "{:#}", run.row);
    assert_eq!(run.row["behaviour"]["protocol_version"], Value::Null);
}

#[test]
fn a_capture_of_real_koda_scores_through_the_runner() {
    let run = score(Path::new(CAPTURED));
    let row = &run.row;
    assert_eq!(row["pass"], true, "{row:#}");
    assert_eq!(row["failure"], Value::Null, "{row:#}");
    assert_eq!(row["events"]["unparsed_line"], Value::Null, "{row:#}");

    let behaviour = &row["behaviour"];
    assert_eq!(behaviour["protocol_version"], 1, "{behaviour:#}");
    assert_eq!(behaviour["compactions"]["attempts"], 4, "{behaviour:#}");
    assert_eq!(behaviour["compactions"]["with_summary"], 1, "{behaviour:#}");
    assert_eq!(behaviour["loop_guard_warnings"], 1, "{behaviour:#}");
    assert_eq!(behaviour["edit_rejections"], 1, "{behaviour:#}");
    assert_eq!(behaviour["delegations"], 1, "{behaviour:#}");
    assert_eq!(row["summary"]["usage"]["api_calls"], 19, "{row:#}");
}
