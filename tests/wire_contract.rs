//! The wire protocol through the runner, verdict included: `koda-eval run`
//! against a fake `koda` that prints a transcript. One is a capture of the real
//! binary (`bin/capture-koda-wire`). No model and no API key.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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

/// A project with one task and a `koda` that prints `stream`, and its run file.
fn project(root: &Path, stream: &Path) -> PathBuf {
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
    run_file
}

fn koda_eval(root: &Path, run_file: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_koda-eval"));
    cmd.arg("--root")
        .arg(root)
        .arg("run")
        .arg(run_file)
        .env("XDG_CACHE_HOME", root.join("cache"));
    cmd
}

/// `koda-eval run` on one task, with a `koda` that prints `stream`.
fn score(stream: &Path) -> Finished {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let run_file = project(root, stream);
    let out = koda_eval(root, &run_file).output().unwrap();
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

/// A `PATH` whose `ps` fails once, the first time it runs after `trigger`
/// exists.
fn flaky_ps(root: &Path, trigger: &Path) -> String {
    let bin = root.join("flaky-ps");
    std::fs::create_dir(&bin).unwrap();
    let path = std::env::var("PATH").unwrap();
    let ps = format!(
        "#!/bin/sh\n\
         if [ -e '{}' ] && mkdir '{}' 2>/dev/null; then\n\
         echo 'not allowed' >&2\n\
         exit 1\n\
         fi\n\
         PATH='{path}' exec ps \"$@\"\n",
        trigger.display(),
        root.join("failed").display(),
    );
    let stub = bin.join("ps");
    std::fs::write(&stub, ps).unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    format!("{}:{path}", bin.display())
}

/// The rows a run saved.
fn saved_rows(root: &Path) -> Vec<Value> {
    let runs: Vec<_> = std::fs::read_dir(root.join("results/wire"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    std::fs::read_to_string(runs[0].join("results.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn a_cleanup_that_cant_be_checked_stops_the_run_ungraded() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let run_file = project(root, Path::new(CAPTURED));
    let run = std::fs::read_to_string(&run_file).unwrap();
    std::fs::write(&run_file, format!("repeats = 2\n{run}")).unwrap();

    let bin = root.join("no-ps");
    std::fs::create_dir(&bin).unwrap();
    let ps = bin.join("ps");
    std::fs::write(&ps, "#!/bin/sh\necho 'not allowed' >&2\nexit 1\n").unwrap();
    std::fs::set_permissions(&ps, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());

    let out = koda_eval(root, &run_file)
        .env("PATH", path)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("stopping the run"), "{stderr}");

    let runs: Vec<_> = std::fs::read_dir(root.join("results/wire"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let rows = std::fs::read_to_string(runs[0].join("results.jsonl")).unwrap();
    assert_eq!(rows.lines().count(), 1, "{rows}");
    let row: Value = serde_json::from_str(rows.trim()).unwrap();
    assert_eq!(row["pass"], Value::Null, "{row:#}");
    assert_eq!(row["check"], Value::Null, "{row:#}");
    let error = row["harness_error"].as_str().unwrap();
    assert!(error.contains("not allowed"), "{error}");
}

#[test]
fn a_failed_cleanup_stops_parallel_workers_before_they_grade() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let run_file = project(root, Path::new(CAPTURED));
    let run = std::fs::read_to_string(&run_file).unwrap();
    std::fs::write(&run_file, format!("repeats = 4\n{run}")).unwrap();

    // The first `koda` finishes at once, and the `ps` that cleans up after it
    // fails. The second is still running then, and its own cleanup succeeds.
    let (lock, fast) = (root.join("lock"), root.join("fast"));
    let koda = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo 'koda 9.9.9'; exit 0; fi\n\
         if mkdir '{}' 2>/dev/null; then touch '{}'; else sleep 2; fi\n\
         cat '{CAPTURED}'\n",
        lock.display(),
        fast.display(),
    );
    std::fs::write(root.join("koda"), koda).unwrap();

    let path = flaky_ps(root, &fast);

    let out = koda_eval(root, &run_file)
        .args(["--jobs", "2"])
        .env("PATH", path)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("stopping the run"), "{stderr}");
    assert!(!stderr.contains("[3/4]"), "a queued run started: {stderr}");

    let rows = saved_rows(root);
    assert_eq!(rows.len(), 2, "{rows:#?}");
    for row in &rows {
        assert_eq!(row["pass"], Value::Null, "{row:#}");
        assert_eq!(row["check"], Value::Null, "{row:#}");
    }
    let errors: Vec<_> = rows
        .iter()
        .map(|r| r["harness_error"].to_string())
        .collect();
    assert!(
        errors.iter().any(|e| e.contains("not allowed")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("not graded")),
        "{errors:?}"
    );
}

#[test]
fn a_run_graded_while_a_cleanup_fails_is_not_scored() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let run_file = project(root, Path::new(CAPTURED));
    let run = std::fs::read_to_string(&run_file).unwrap();
    std::fs::write(&run_file, format!("repeats = 2\n{run}")).unwrap();

    // One run reaches grading and marks it. Only then does the other `koda`
    // exit, so its cleanup fails while the first run is being graded.
    let (lock, grading) = (root.join("lock"), root.join("grading"));
    let task = TASK.replace(
        "[check]\n",
        &format!(
            "[check]\n\
             hidden = [{{ file = \"README.md\", test = \"hidden/test.rs\" }}]\n\
             command = \"touch '{}'; sleep 2; echo 'test eval_hidden::a ... ok'\"\n\
             timeout_secs = 30\n",
            grading.display(),
        ),
    );
    std::fs::write(root.join("tasks/wire/task.toml"), task).unwrap();
    std::fs::write(root.join("tasks/wire/hidden/test.rs"), "").unwrap();

    let koda = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo 'koda 9.9.9'; exit 0; fi\n\
         if ! mkdir '{}' 2>/dev/null; then\n\
         n=0; while [ ! -e '{}' ] && [ $n -lt 100 ]; do sleep 0.1; n=$((n + 1)); done\n\
         fi\n\
         cat '{CAPTURED}'\n",
        lock.display(),
        grading.display(),
    );
    std::fs::write(root.join("koda"), koda).unwrap();
    let path = flaky_ps(root, &grading);

    let out = koda_eval(root, &run_file)
        .args(["--jobs", "2"])
        .env("PATH", path)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");

    let rows = saved_rows(root);
    assert_eq!(rows.len(), 2, "{stderr}\n{rows:#?}");
    let graded = rows
        .iter()
        .find(|r| r["check"] != Value::Null)
        .unwrap_or_else(|| panic!("no graded row: {rows:#?}"));
    assert_eq!(graded["check"]["hidden"]["hidden_ok"], 1, "{graded:#}");
    assert_eq!(graded["pass"], Value::Null, "{graded:#}");
    let error = graded["harness_error"].as_str().unwrap();
    assert!(error.contains("not trusted"), "{error}");
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
