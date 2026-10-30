//! End to end through a real `koda` binary driven by koda's mock provider.
//!
//! Needs a koda build with the mock provider, which only exists under the
//! `test-support` feature:
//!
//! ```bash
//! cargo build -p koda-cli --features koda-core/test-support   # in ~/repo/koda
//! KODA_EVAL_TEST_BIN=~/repo/koda/target/debug/koda cargo test --test mock_e2e
//! ```
//!
//! Without `KODA_EVAL_TEST_BIN` the test is skipped.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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

#[test]
fn a_fix_passes_and_giving_up_fails() {
    let Some(koda) = std::env::var_os("KODA_EVAL_TEST_BIN").map(PathBuf::from) else {
        eprintln!("skipped: set KODA_EVAL_TEST_BIN to a koda built with the mock provider");
        return;
    };

    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tasks/kata-largest"),
        &root.join("tasks/kata-largest"),
    );

    let fixes = r#"[{"tool":"Read","args":{"file_path":"src/lib.rs"}},{"tool":"Edit","args":{"file_path":"src/lib.rs","replacements":[{"old_str":"pub fn largest<T>(","new_str":"pub fn largest<T: PartialOrd>("}]}},{"text":"Added the PartialOrd bound."}]"#;
    let gives_up = r#"[{"text":"I can't fix this."}]"#;
    let run_file = root.join("e2e.toml");
    std::fs::write(
        &run_file,
        format!(
            r#"name = "e2e"
tasks = ["kata-largest"]

[[variants]]
name = "koda"
bin = {koda:?}

[[models]]
name = "fixes"
provider = "mock"
env = {{ KODA_MOCK_RESPONSES = {fixes:?} }}

[[models]]
name = "gives-up"
provider = "mock"
env = {{ KODA_MOCK_RESPONSES = {gives_up:?} }}
"#
        ),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_koda-eval"))
        .arg("--root")
        .arg(root)
        .arg("run")
        .arg(&run_file)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.contains("koda / fixes: 1 of 1 passed"), "{stdout}");
    assert!(
        stdout.contains("koda / gives-up: 0 of 1 passed"),
        "{stdout}"
    );

    let runs: Vec<PathBuf> = std::fs::read_dir(root.join("results/e2e"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(runs.len(), 1);
    let dir = &runs[0];
    assert!(dir.join("meta.json").is_file());

    let rows: Vec<Value> = std::fs::read_to_string(dir.join("results.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), 2);

    let fixed = rows.iter().find(|r| r["model"] == "fixes").unwrap();
    assert_eq!(fixed["pass"], true, "{fixed:#}");
    assert_eq!(fixed["summary"]["tool_calls"]["Edit"], 1, "{fixed:#}");
    assert_eq!(fixed["check"]["hidden"]["hidden_ok"], 3, "{fixed:#}");
    assert_eq!(fixed["check"]["transcript"], Value::Null, "{fixed:#}");
    assert_eq!(fixed["config"]["engine"], "mock", "{fixed:#}");
    assert_eq!(fixed["config"]["trust"], "auto", "{fixed:#}");
    assert_eq!(fixed["config"]["cluster"], "kata", "{fixed:#}");
    assert_eq!(fixed["config"]["wall_secs"], 300, "{fixed:#}");
    assert_eq!(fixed["behaviour"]["turn_end_kind"], "complete", "{fixed:#}");
    assert_eq!(fixed["behaviour"]["edit_rejections"], 0, "{fixed:#}");
    assert_eq!(fixed["behaviour"]["time_limit_hit"], false, "{fixed:#}");
    // The mock provider's window is tiny, so koda compacts; the count is
    // koda's business, the signal's presence is ours.
    assert!(
        fixed["behaviour"]["compactions"]["attempts"].is_u64(),
        "{fixed:#}"
    );
    assert_eq!(fixed["diff"]["files"], 1, "{fixed:#}");
    let artifacts = dir.join(fixed["artifacts"].as_str().unwrap());
    assert!(artifacts.join("transcript.ndjson").is_file());
    assert!(
        std::fs::read_to_string(artifacts.join("diff.patch"))
            .unwrap()
            .contains("+pub fn largest<T: PartialOrd>(")
    );

    let gave_up = rows.iter().find(|r| r["model"] == "gives-up").unwrap();
    assert_eq!(gave_up["pass"], false, "{gave_up:#}");
    assert_eq!(gave_up["failure"], "did not compile", "{gave_up:#}");
    assert_eq!(gave_up["diff"]["files"], 0, "{gave_up:#}");

    // The run printed its report, and `report <run name>` finds the same
    // results again.
    assert!(
        stdout.contains("| koda | fixes | 1/1 | 100% | 1/1 |"),
        "{stdout}"
    );
    assert!(
        stdout.contains("| koda | gives-up | 0/1 | 0% | 0/1 |"),
        "{stdout}"
    );
    assert!(dir.join("report.md").is_file());
    let again = Command::new(env!("CARGO_BIN_EXE_koda-eval"))
        .arg("--root")
        .arg(root)
        .args(["report", "e2e"])
        .output()
        .unwrap();
    let again_out = String::from_utf8_lossy(&again.stdout);
    assert!(again.status.success(), "{again_out}");
    assert!(
        again_out.contains("| kata-largest | 1/1 | 0/1 |"),
        "{again_out}"
    );
}
