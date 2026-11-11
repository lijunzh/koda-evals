//! `bin/qwen-as-koda`: the koda events it makes of Qwen Code's stream, from a
//! recorded one, and on macOS the settings, environment, stubs and sandbox it
//! runs a fake `qwen` with. No model and no GPU.

use std::collections::BTreeMap;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/qwen-stream.ndjson"
);

fn wrapper() -> Command {
    Command::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/qwen-as-koda"))
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

fn translate(extra: &[&str]) -> Vec<Value> {
    let out = wrapper()
        .args(["--translate", FIXTURE])
        .args(extra)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    events(&out)
}

#[test]
fn the_stream_becomes_koda_events() {
    let ev = translate(&[]);
    assert_eq!(ev[0]["type"], "turn_start");
    let calls: BTreeMap<&str, &Value> = of_type(&ev, "tool_call_start")
        .into_iter()
        .map(|e| (e["id"].as_str().unwrap(), e))
        .collect();

    // read_file's 0-based offset is koda's 1-based start line, and paths are
    // relative to the project.
    assert_eq!(calls["t1"]["name"], "Read");
    assert_eq!(calls["t1"]["args"]["file_path"], "src/lib.rs");
    assert_eq!(calls["t1"]["args"]["start_line"], 11);
    assert_eq!(calls["t1"]["args"]["num_lines"], 20);
    assert_eq!(calls["t2"]["name"], "Edit");
    assert_eq!(
        calls["t2"]["args"]["replacements"][0]["new_str"],
        "fn parse() -> u8 { 0 }"
    );
    assert!(calls["t2"]["args"].get("record_as_artifact").is_none());
    assert_eq!(calls["t3"]["name"], "Bash");
    // A deferred tool called through tool_call is the tool it names.
    assert_eq!(calls["t4"]["name"], "TodoWrite");
    assert_eq!(calls["t6"]["name"], "InvokeAgent");
    assert_eq!(calls["t6"]["args"]["agent_name"], "Explore");
    // The sub-agent's calls are marked and counted apart.
    assert_eq!(calls["t7"]["name"], "Grep");
    assert_eq!(calls["t7"]["is_sub_agent"], true);
    assert_eq!(calls["t7"]["args"]["file_path"], "src");

    let results: BTreeMap<&str, &str> = of_type(&ev, "tool_call_result")
        .into_iter()
        .map(|e| (e["id"].as_str().unwrap(), e["output"].as_str().unwrap()))
        .collect();
    assert!(results["t2"].starts_with("Error: "), "{}", results["t2"]);
    // A failed command gets koda's exit-code head; a successful one, which
    // Qwen Code reports as its bare output, is left as is.
    assert!(
        results["t3"].starts_with("Exit code: 101 | "),
        "{}",
        results["t3"]
    );
    assert_eq!(results["t5"], "test result: ok. 2 passed");

    let requests = of_type(&ev, "approval_request");
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert_eq!(requests[0]["effect"], "Destructive");
    assert_eq!(requests[0]["detail"], "git reset --hard && cargo test");

    assert_eq!(of_type(&ev, "sub_agent_text").len(), 1);
    let summary = ev.last().unwrap();
    assert_eq!(summary["type"], "run_summary");
    assert_eq!(summary["success"], true);
    assert_eq!(summary["turn_end"]["kind"], "complete");
    assert_eq!(
        summary["response"],
        "Fixed parse in src/lib.rs; all tests pass."
    );
    assert_eq!(summary["tool_calls"]["Bash"], 2);
    assert_eq!(summary["tool_calls"]["InvokeAgent"], 1);
    assert_eq!(summary["sub_agent_tool_calls"]["Grep"], 1);
    assert_eq!(summary["usage"]["prompt_tokens"], 5000);
    assert_eq!(summary["usage"]["cache_read_tokens"], 3000);
    assert_eq!(summary["usage"]["completion_tokens"], 400);
    assert_eq!(summary["provider"], "qwen-code");
}

#[test]
fn safe_mode_asks_before_file_changes() {
    let ev = translate(&["--mode", "safe"]);
    let effects: Vec<&str> = of_type(&ev, "approval_request")
        .iter()
        .map(|e| e["effect"].as_str().unwrap())
        .collect();
    assert_eq!(effects, ["LocalMutation", "Destructive"]);
}

#[test]
fn a_run_stopped_at_the_time_limit_ends_cancelled() {
    let ev = translate(&["--exit", "130", "--cancelled"]);
    let summary = ev.last().unwrap();
    assert_eq!(summary["success"], false);
    assert_eq!(summary["turn_end"]["kind"], "cancelled");
    assert_eq!(ev[ev.len() - 2]["reason"]["kind"], "cancelled");
}

/// A temp HOME whose `.cargo/bin/qwen` records what it was started with, then
/// prints the fixture. The wrapper's PATH has that directory.
#[cfg(target_os = "macos")]
fn fake_home() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join(".cargo/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let root = home.path().display();
    let outside = outside(home.path()).display().to_string();
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo '9.9.9'; exit 0; fi\n\
         for a in \"$@\"; do printf '%s\\n' \"$a\"; done > {root}/config/args\n\
         env > {root}/config/env\n\
         codex >/dev/null 2>&1; echo $? > {root}/config/codex-exit\n\
         touch '{outside}' 2>/dev/null; echo $? > {root}/config/outside-exit\n\
         cat '{FIXTURE}'\n"
    );
    let path = bin.join("qwen");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::create_dir_all(home.path().join("config")).unwrap();
    std::fs::create_dir_all(home.path().join("proj")).unwrap();
    home
}

/// A path the sandbox must not let qwen write: outside the temp directories,
/// which it allows by design, so not beside the temp HOME.
#[cfg(target_os = "macos")]
fn outside(home: &Path) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!(
        "qwen-sandbox-probe-{}",
        home.file_name().unwrap().to_string_lossy()
    ))
}

#[cfg(target_os = "macos")]
fn run(home: &Path, args: &[&str]) -> Output {
    wrapper()
        .args(args)
        .current_dir(home.join("proj"))
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("CARGO_TARGET_DIR", home.join("target"))
        .env("KODA_MAX_CONTEXT_TOKENS", "262144")
        .env("KODA_REASONING_EFFORT", "xhigh")
        .env("ANTHROPIC_API_KEY", "not-for-qwen")
        .output()
        .unwrap()
}

#[cfg(target_os = "macos")]
#[test]
fn qwen_gets_its_settings_a_clean_env_and_a_sandbox() {
    let home = fake_home();
    let h = home.path();
    let out = run(
        h,
        &[
            "-p",
            "-fix it",
            "--model",
            "qwen38",
            "--base-url",
            "http://localhost:9/v1",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(events(&out).last().unwrap()["success"], true);

    let settings: Value = serde_json::from_str(
        &std::fs::read_to_string(h.join("config/qwen/settings.json")).unwrap(),
    )
    .unwrap();
    let model = &settings["modelProviders"]["openai"][0];
    assert_eq!(settings["model"]["name"], "qwen38");
    assert_eq!(model["id"], "qwen38");
    assert_eq!(model["baseUrl"], "http://localhost:9/v1");
    assert_eq!(model["generationConfig"]["contextWindowSize"], 262144);
    assert_eq!(
        model["generationConfig"]["extra_body"]["reasoning_effort"],
        "xhigh"
    );
    assert_eq!(settings["context"]["fileName"][0], "AGENTS.md");

    let args = std::fs::read_to_string(h.join("config/args")).unwrap();
    let args: Vec<&str> = args.lines().collect();
    for flag in ["stream-json", "yolo", "web_fetch,web_search"] {
        assert!(args.contains(&flag), "missing {flag}: {args:?}");
    }
    // A prompt that starts with "-" stays the prompt.
    assert_eq!(args.last(), Some(&"--prompt=-fix it"));

    let env = std::fs::read_to_string(h.join("config/env")).unwrap();
    assert!(!env.contains("not-for-qwen"), "an API key reached qwen");
    assert!(env.contains(&format!("QWEN_HOME={}", h.join("config/qwen").display())));
    assert!(!env.contains(".local/bin"), "PATH reaches ~/.local/bin");
    // Codex is a stub, and writes outside the allowed roots fail.
    assert_eq!(
        std::fs::read_to_string(h.join("config/codex-exit"))
            .unwrap()
            .trim(),
        "127"
    );
    assert_ne!(
        std::fs::read_to_string(h.join("config/outside-exit"))
            .unwrap()
            .trim(),
        "0"
    );
    assert!(!outside(h).exists());
    // Qwen Code's own events are kept, on stderr.
    assert!(String::from_utf8_lossy(&out.stderr).contains("\"subtype\":\"init\""));
}

#[cfg(target_os = "macos")]
#[test]
fn version_names_qwen_code() {
    let home = fake_home();
    let out = run(home.path(), &["--version"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "qwen-as-koda qwen-code 9.9.9"
    );
}
