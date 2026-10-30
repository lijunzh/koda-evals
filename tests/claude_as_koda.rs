//! `bin/claude-as-koda` against a fake `claude`: the environment it builds,
//! the flags it passes, and the koda events it writes. No model call and no
//! quota: `HOME` points at a temp directory whose `.local/bin/claude` is the
//! script below, and the wrapper looks there first.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-stream.ndjson"
);

/// A temp `HOME` with a fake `claude` that records its environment names
/// and arguments, then prints `stream`.
fn fake_home(stream: &Path) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join(".local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let root = home.path().display();
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo '9.9.9 (Claude Code)'; exit 0; fi\n\
         env | cut -d= -f1 > {root}/env-names\n\
         for a in \"$@\"; do printf '%s\\n' \"$a\"; done > {root}/args\n\
         cat '{}'\n",
        stream.display()
    );
    let path = bin.join("claude");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    home
}

fn wrapper(home: &Path, args: &[&str]) -> Output {
    Command::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/claude-as-koda"))
        .args(args)
        .current_dir(home)
        .env("HOME", home)
        .env("ANTHROPIC_API_KEY", "sk-ant-not-a-real-key")
        .env("ANTHROPIC_AUTH_TOKEN", "not-real")
        .env("ANTHROPIC_BASE_URL", "http://localhost:1")
        .env("OPENAI_API_KEY", "not-real")
        .env("CLAUDE_CODE_USE_BEDROCK", "1")
        .env("CLAUDE_CODE_OAUTH_TOKEN", "not-real")
        .env("CARGO_REGISTRY_TOKEN", "not-real")
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
fn version_names_claude_code() {
    let home = fake_home(Path::new(FIXTURE));
    let out = wrapper(home.path(), &["--version"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "claude-as-koda 9.9.9 (Claude Code)"
    );
}

#[test]
fn no_key_reaches_the_child_and_values_are_not_logged() {
    let home = fake_home(Path::new(FIXTURE));
    let out = wrapper(
        home.path(),
        &["-p", "fix it", "--model", "opus", "--provider", "anthropic"],
    );
    assert!(out.status.success());
    let names = std::fs::read_to_string(home.path().join("env-names")).unwrap();
    for name in names.lines() {
        assert!(
            !["ANTHROPIC", "CLAUDE", "OPENAI", "TOKEN", "API_KEY"]
                .iter()
                .any(|bad| name.contains(bad)),
            "{name} reached the child"
        );
    }
    assert!(names.lines().any(|n| n == "CARGO_TARGET_DIR"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("child env names:"));
    assert!(!stderr.contains("sk-ant-not-a-real-key"));
}

#[test]
fn flags_keep_the_run_on_defaults_and_inside_the_project() {
    let home = fake_home(Path::new(FIXTURE));
    wrapper(
        home.path(),
        &["-p", "fix it", "--model", "opus", "--base-url", "http://x"],
    );
    let args = std::fs::read_to_string(home.path().join("args")).unwrap();
    let args: Vec<&str> = args.lines().collect();
    for flag in [
        "--safe-mode",
        "--strict-mcp-config",
        "--no-session-persistence",
        "--disable-slash-commands",
        "dontAsk",
        "opus",
    ] {
        assert!(args.contains(&flag), "missing {flag}");
    }
    assert!(!args.contains(&"http://x"));
    assert!(!args.contains(&"bypassPermissions"));
    let at = |flag: &str| args[args.iter().position(|a| *a == flag).unwrap() + 1];
    assert!(at("--allowedTools").contains("Edit(./**)"));
    assert!(!at("--tools").contains("Web"));
    let settings: Value = serde_json::from_str(at("--settings")).unwrap();
    let sandbox = &settings["sandbox"];
    assert_eq!(sandbox["enabled"], true);
    assert_eq!(sandbox["allowUnsandboxedCommands"], false);
    let writable = sandbox["filesystem"]["allowWrite"].to_string();
    assert!(writable.contains("/tmp/eval-target"), "{writable}");
}

#[test]
fn the_stream_becomes_koda_events() {
    let home = fake_home(Path::new(FIXTURE));
    let out = wrapper(home.path(), &["-p", "fix it", "--mode", "safe"]);
    let ev = events(&out);

    let calls: BTreeMap<&str, &Value> = of_type(&ev, "tool_call_start")
        .into_iter()
        .map(|e| (e["id"].as_str().unwrap(), e))
        .collect();
    let read = &calls["t1"]["args"];
    assert_eq!(read["start_line"], 20);
    assert_eq!(read["num_lines"], 40);
    assert!(read.get("offset").is_none());
    assert_eq!(
        calls["t2"]["args"]["replacements"][0]["old_str"],
        "not there"
    );
    assert_eq!(calls["t5"]["name"], "InvokeAgent");
    assert_eq!(calls["t5"]["args"]["agent_name"], "general-purpose");
    assert_eq!(calls["t5"]["is_sub_agent"], false);
    assert_eq!(calls["t6"]["is_sub_agent"], true);
    assert_eq!(calls["t6"]["args"]["file_path"], "src");

    let results = of_type(&ev, "tool_call_result");
    let edit = results.iter().find(|r| r["id"] == "t2").unwrap();
    assert_eq!(edit["name"], "Edit");
    assert!(
        edit["output"]
            .as_str()
            .unwrap()
            .starts_with("Error: String")
    );

    let effects: Vec<&str> = of_type(&ev, "approval_request")
        .iter()
        .map(|e| e["effect"].as_str().unwrap())
        .collect();
    assert_eq!(effects, ["LocalMutation", "Destructive", "LocalMutation"]);

    // Two `result` events, one denial: reported once.
    let blocked = of_type(&ev, "action_blocked");
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["tool_name"], "Write");

    // The child's text isn't the final message; the last thing out is.
    assert_eq!(of_type(&ev, "turn_end").len(), 1);
    let done = ev.iter().rposition(|e| e["type"] == "text_done").unwrap();
    assert!(done > ev.iter().position(|e| e["id"] == "t6").unwrap());
    let summary = ev.last().unwrap();
    assert_eq!(summary["type"], "run_summary");
    assert_eq!(summary["success"], true);
    assert_eq!(summary["turn_end"]["kind"], "complete");
    assert_eq!(
        summary["response"],
        "Fixed `f` in src/big.rs. Worker confirmed it."
    );
    assert_eq!(summary["api_key_source"], "none");
    assert_eq!(summary["tool_calls"]["InvokeAgent"], 1);
    assert_eq!(summary["sub_agent_tool_calls"]["Grep"], 1);

    // Cumulative total; main is the sum of the segments; children the rest.
    assert_eq!(summary["usage"]["prompt_tokens"], 14);
    assert_eq!(summary["usage"]["completion_tokens"], 718);
    assert_eq!(summary["usage"]["api_calls"], 9);
    let main = &summary["usage_by_agent"]["main"];
    assert_eq!(main["prompt_tokens"], 10);
    assert_eq!(main["completion_tokens"], 477);
    assert_eq!(main["api_calls"], 7);
    assert_eq!(
        summary["usage_by_agent"]["sub_agents"]["completion_tokens"],
        241
    );
}

#[test]
fn auto_mode_raises_only_destructive_approvals() {
    let home = fake_home(Path::new(FIXTURE));
    let ev = events(&wrapper(home.path(), &["-p", "fix it", "--mode", "auto"]));
    let requests = of_type(&ev, "approval_request");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["effect"], "Destructive");
    assert_eq!(requests[0]["detail"], "git reset --hard && cargo test");
}

#[test]
fn a_run_not_on_the_subscription_is_stopped() {
    let scratch = tempfile::tempdir().unwrap();
    let stream = scratch.path().join("api.ndjson");
    std::fs::write(
        &stream,
        r#"{"type":"system","subtype":"init","session_id":"s","model":"m","apiKeySource":"ANTHROPIC_API_KEY"}
{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"never read"}]},"parent_tool_use_id":null}
"#,
    )
    .unwrap();
    let home = fake_home(&stream);
    let out = wrapper(home.path(), &["-p", "fix it"]);
    assert_eq!(out.status.code(), Some(3));
    let ev = events(&out);
    assert_eq!(of_type(&ev, "text_done").len(), 0);
    let summary = ev.last().unwrap();
    assert_eq!(summary["success"], false);
    assert_eq!(summary["turn_end"]["kind"], "error");
}

#[test]
fn an_error_result_ends_the_turn_as_an_error() {
    let scratch = tempfile::tempdir().unwrap();
    let stream = scratch.path().join("limit.ndjson");
    std::fs::write(
        &stream,
        r#"{"type":"system","subtype":"init","session_id":"s","model":"m","apiKeySource":"none"}
{"type":"rate_limit_event","rate_limit_info":{"status":"rejected"}}
{"type":"result","subtype":"success","is_error":true,"result":"limit reached","api_error_status":429,"usage":{},"modelUsage":{},"permission_denials":[]}
"#,
    )
    .unwrap();
    let home = fake_home(&stream);
    let ev = events(&wrapper(home.path(), &["-p", "fix it"]));
    assert_eq!(of_type(&ev, "warn").len(), 1);
    let summary = ev.last().unwrap();
    assert_eq!(summary["turn_end"]["kind"], "error");
    assert_eq!(summary["rate_limited"], true);
    assert_eq!(summary["error"], "limit reached");
}
