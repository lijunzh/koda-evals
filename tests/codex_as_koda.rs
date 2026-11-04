//! `bin/codex-as-koda` against a fake `codex`: the home and flags it gives
//! Codex, and the koda events it writes. No model and no GPU: `HOME` points at
//! a temp directory whose `.local/bin/codex` is the script below, and the
//! wrapper looks there first.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/codex-stream.ndjson"
);

/// A temp `HOME` with a fake `codex` that records its `CODEX_HOME`, base URL
/// and arguments, then prints `stream`.
fn fake_home(stream: &Path) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join(".local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let root = home.path().display();
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo 'codex-cli 9.9.9'; exit 0; fi\n\
         echo \"$CODEX_HOME\" > {root}/codex-home\n\
         echo \"$CODEX_OSS_BASE_URL\" > {root}/base-url\n\
         for a in \"$@\"; do printf '%s\\n' \"$a\"; done > {root}/args\n\
         cat '{}'\n",
        stream.display()
    );
    let path = bin.join("codex");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    home
}

fn wrapper(home: &Path, args: &[&str]) -> Output {
    Command::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/codex-as-koda"))
        .args(args)
        .current_dir(home)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("KODA_MAX_CONTEXT_TOKENS", "131072")
        .env("CARGO_TARGET_DIR", "/tmp/eval-target")
        .output()
        .unwrap()
}

fn events(out: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn of_type<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
    events.iter().filter(|e| e["type"] == kind).collect()
}

#[test]
fn version_names_codex() {
    let home = fake_home(Path::new(FIXTURE));
    let out = wrapper(home.path(), &["--version"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "codex-cli 9.9.9"
    );
}

#[test]
fn codex_gets_a_fresh_home_the_window_and_the_server() {
    let home = fake_home(Path::new(FIXTURE));
    let out = wrapper(
        home.path(),
        &[
            "-p",
            "fix it",
            "--model",
            "ornith",
            "--base-url",
            "http://localhost:1234/v1",
            "--provider",
            "lm-studio",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let codex_home = std::fs::read_to_string(home.path().join("codex-home")).unwrap();
    assert_eq!(
        codex_home.trim(),
        home.path().join("config/codex").display().to_string()
    );
    assert!(home.path().join("config/codex").is_dir());
    let base = std::fs::read_to_string(home.path().join("base-url")).unwrap();
    assert_eq!(base.trim(), "http://localhost:1234/v1");

    let args = std::fs::read_to_string(home.path().join("args")).unwrap();
    let args: Vec<&str> = args.lines().collect();
    for flag in [
        "exec",
        "--oss",
        "--json",
        "workspace-write",
        "approval_policy=never",
        // A fresh CODEX_HOME defaults to the openai provider, and Codex then
        // refuses an --oss turn.
        "model_provider=lmstudio",
    ] {
        assert!(args.contains(&flag), "missing {flag}: {args:?}");
    }
    let at = |flag: &str| args[args.iter().position(|a| *a == flag).unwrap() + 1];
    assert_eq!(at("-m"), "ornith");
    assert!(args.contains(&"model_context_window=131072"), "{args:?}");
    assert!(
        args.iter()
            .any(|a| a.starts_with("sandbox_workspace_write.writable_roots=")
                && a.contains("/tmp/eval-target")),
        "{args:?}"
    );
    assert_eq!(args.last(), Some(&"fix it"));
    // Codex's own events are kept, on stderr.
    assert!(String::from_utf8_lossy(&out.stderr).contains("\"thread.started\""));
}

#[test]
fn the_stream_becomes_koda_events() {
    let home = fake_home(Path::new(FIXTURE));
    let ev = events(&wrapper(home.path(), &["-p", "fix it", "--mode", "auto"]));

    let calls: BTreeMap<&str, &Value> = of_type(&ev, "tool_call_start")
        .into_iter()
        .map(|e| (e["id"].as_str().unwrap(), e))
        .collect();
    // Unwrapped from Codex's `/bin/zsh -lc`.
    assert_eq!(calls["item_2"]["name"], "Bash");
    assert_eq!(calls["item_2"]["args"]["command"], "cargo test -p demo");
    // An apply_patch command is an Edit, not Bash.
    assert_eq!(calls["item_4.0"]["name"], "Edit");
    assert_eq!(calls["item_4.0"]["args"]["file_path"], "src/lib.rs");
    // A file_change is one call per file, by kind.
    assert_eq!(calls["item_5.0"]["name"], "Write");
    assert_eq!(calls["item_5.1"]["name"], "Delete");
    // A command that completes with no start event is still announced.
    assert_eq!(calls["item_6"]["args"]["command"], "cargo test");

    let results: BTreeMap<&str, &str> = of_type(&ev, "tool_call_result")
        .into_iter()
        .map(|e| (e["id"].as_str().unwrap(), e["output"].as_str().unwrap()))
        .collect();
    assert_eq!(results.len(), calls.len());
    assert!(
        results["item_2"].starts_with("Exit code: 101 |"),
        "{}",
        results["item_2"]
    );
    assert!(results["item_6"].starts_with("Exit code: 0 |"));

    // Only the destructive command: the patch body's `$(` isn't a command.
    let requests = of_type(&ev, "approval_request");
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert_eq!(requests[0]["effect"], "Destructive");
    assert_eq!(requests[0]["detail"], "git reset --hard && cargo test");

    assert_eq!(of_type(&ev, "warn").len(), 1);
    let summary = ev.last().unwrap();
    assert_eq!(summary["type"], "run_summary");
    assert_eq!(summary["success"], true);
    assert_eq!(summary["turn_end"]["kind"], "complete");
    assert_eq!(
        summary["response"],
        "Fixed the overflow in src/lib.rs; all tests pass."
    );
    assert_eq!(summary["tool_calls"]["Bash"], 3);
    assert_eq!(summary["tool_calls"]["Edit"], 1);
    assert_eq!(summary["usage"]["prompt_tokens"], 5000);
    assert_eq!(summary["usage"]["cache_read_tokens"], 3000);
    assert_eq!(summary["usage"]["completion_tokens"], 400);
    assert_eq!(summary["usage"]["thinking_tokens"], 250);
}

#[test]
fn safe_mode_asks_before_every_file_change() {
    let home = fake_home(Path::new(FIXTURE));
    let ev = events(&wrapper(home.path(), &["-p", "fix it", "--mode", "safe"]));
    let effects: Vec<&str> = of_type(&ev, "approval_request")
        .iter()
        .map(|e| e["effect"].as_str().unwrap())
        .collect();
    assert_eq!(
        effects,
        [
            "Destructive",
            "LocalMutation",
            "LocalMutation",
            "LocalMutation"
        ]
    );
}

#[test]
fn a_run_without_turn_completed_ends_as_an_error() {
    let scratch = tempfile::tempdir().unwrap();
    let stream = scratch.path().join("cut.ndjson");
    std::fs::write(
        &stream,
        r#"{"type":"thread.started","thread_id":"th-2"}
{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"Looking."}}
{"type":"turn.failed","error":{"message":"stream disconnected before completion"}}
"#,
    )
    .unwrap();
    let home = fake_home(&stream);
    let ev = events(&wrapper(home.path(), &["-p", "fix it"]));
    let summary = ev.last().unwrap();
    assert_eq!(summary["success"], false);
    assert_eq!(summary["turn_end"]["kind"], "error");
    assert_eq!(summary["error"], "stream disconnected before completion");
}

/// `src/check.rs` grades the committed translation of the fixture, so the
/// wrapper must still produce exactly that.
#[test]
fn the_committed_translation_is_current() {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/codex-as-koda");
    let out = Command::new(bin)
        .args(["--translate", FIXTURE])
        .output()
        .unwrap();
    assert!(out.status.success());
    let committed = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/codex-as-koda.ndjson"
    ))
    .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        committed,
        "regenerate with bin/codex-as-koda --translate tests/fixtures/codex-stream.ndjson"
    );
}

/// Translate `events` (one Codex event per line) with `extra` arguments.
fn translate(codex_events: &[Value], extra: &[&str]) -> Vec<Value> {
    let tmp = tempfile::tempdir().unwrap();
    let stream = tmp.path().join("stderr.log");
    let mut text =
        String::from("codex-as-koda: CODEX_HOME=/x\nReading additional input from stdin...\n");
    for e in codex_events {
        text.push_str(&format!("{e}\n"));
    }
    std::fs::write(&stream, text).unwrap();
    let out = Command::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/codex-as-koda"))
        .arg("--translate")
        .arg(&stream)
        .args(extra)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    events(&out)
}

fn command(id: &str, command: &str) -> Value {
    serde_json::json!({"type": "item.completed", "item": {"id": id, "type": "command_execution",
        "command": command, "aggregated_output": "Success.", "exit_code": 0, "status": "completed"}})
}

#[test]
fn apply_patch_after_other_commands_is_still_an_edit() {
    let patch = "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-a\n+b\n*** End Patch\n";
    let ev = translate(
        &[
            // Ornith writes the patch to a file, then applies it on a later line.
            command(
                "item_1",
                &format!("cat > /tmp/fix.patch << 'EOF'\n{patch}EOF\napply_patch < /tmp/fix.patch"),
            ),
            command(
                "item_2",
                &format!("cd /work/project && apply_patch \"{patch}\""),
            ),
            // Naming it isn't running it.
            command("item_3", "echo apply_patch is Codex's edit tool"),
        ],
        &[],
    );
    let names: Vec<&str> = of_type(&ev, "tool_call_start")
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Edit", "Edit", "Bash"], "{ev:?}");
    // The patch body written by `cat` isn't classified as a command.
    assert!(of_type(&ev, "approval_request").is_empty(), "{ev:?}");
}

#[test]
fn a_translated_run_ends_as_the_live_one_did() {
    let message = serde_json::json!({"type": "item.completed",
        "item": {"id": "item_1", "type": "agent_message", "text": "Still looking."}});
    let ev = translate(&[message], &["--exit", "1", "--cancelled"]);
    let end = of_type(&ev, "turn_end");
    assert_eq!(end[0]["reason"]["kind"], "cancelled", "{ev:?}");
    assert_eq!(ev.last().unwrap()["error"], "codex exited 1");
}
