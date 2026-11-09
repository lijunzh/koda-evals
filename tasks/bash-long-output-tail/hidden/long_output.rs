async fn eval_run(command: &str, max_lines: usize) -> ShellOutput {
    let tmp = tempfile::tempdir().unwrap();
    run_shell_command(
        tmp.path(),
        &serde_json::json!({ "command": command }),
        max_lines,
        &BgRegistry::new(),
        None,
        &crate::trust::TrustMode::Safe,
        &koda_sandbox::SandboxPolicy::strict_default(),
        None,
        None,
        None,
    )
    .await
    .unwrap()
}

/// Past the cap, the start and the end of each stream are kept and the
/// middle is dropped.
#[tokio::test]
async fn a_long_output_keeps_its_start_and_its_end() {
    let out = eval_run(
        "seq 1 5000; echo 'test result: FAILED. 1 failed' >&2",
        50,
    )
    .await;
    let s = &out.summary;
    assert!(s.contains("\n1\n2\n"), "the start: {s}");
    assert!(s.contains("\n5000"), "the last line: {s}");
    assert!(s.contains("test result: FAILED"), "stderr's end: {s}");
    assert!(!s.contains("\n2500\n"), "the middle is dropped: {s}");
    assert!(s.len() < 64 * 1024, "bounded: {} bytes", s.len());
}

/// A long stderr keeps its last error even when stdout is short.
#[tokio::test]
async fn a_long_stderr_keeps_its_last_error() {
    let out = eval_run(
        "echo building; for i in $(seq 1 3000); do echo \"warning: $i\" >&2; done; echo 'error: could not compile' >&2",
        50,
    )
    .await;
    let s = &out.summary;
    assert!(s.contains("building"), "{s}");
    assert!(s.contains("warning: 1\n"), "stderr's start: {s}");
    assert!(s.contains("error: could not compile"), "stderr's end: {s}");
    assert!(!s.contains("warning: 1500\n"), "the middle is dropped: {s}");
}

/// Output under the cap is unchanged: every line, no marker needed.
#[tokio::test]
async fn a_short_output_is_kept_whole() {
    let out = eval_run("seq 1 30", 50).await;
    for n in 1..=30 {
        let n = n.to_string();
        assert!(out.summary.lines().any(|l| l.trim() == n), "{n}: {}", out.summary);
    }
}
