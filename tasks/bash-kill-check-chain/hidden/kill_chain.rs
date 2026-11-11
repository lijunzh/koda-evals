async fn eval_bash(registry: &BgRegistry, command: &str, background: bool) -> ShellOutput {
    run_shell_command(
        tempfile::tempdir().unwrap().path(),
        &serde_json::json!({"command": command, "background": background}),
        256,
        registry,
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

async fn eval_start(registry: &BgRegistry, command: &str) -> u32 {
    eval_bash(registry, command, true).await;
    registry.snapshot().last().unwrap().pid
}

/// `kill -0 PID && echo alive || echo gone` on Koda's own background process
/// needs no approval and runs: the echo that applies is printed after it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_chained_check_on_an_own_background_process_runs() {
    let registry = BgRegistry::new();
    let pid = eval_start(&registry, "sleep 60").await;
    let check = format!(r#"kill -0 {pid} && echo "alive" || echo "gone""#);
    let args = serde_json::json!({ "command": check });
    assert!(signals_only_own_background(&args, &registry, None), "{check}");

    let up = eval_bash(&registry, &check, false).await;
    assert!(up.success, "{}", up.summary);
    assert!(up.summary.contains("alive"), "{}", up.summary);
    assert!(!up.summary.contains("gone"), "{}", up.summary);

    registry.kill(pid);
    let down = eval_bash(&registry, &check, false).await;
    assert!(down.success, "the || echo makes it succeed: {}", down.summary);
    assert!(down.summary.contains("gone"), "{}", down.summary);

    let bare = eval_bash(&registry, &format!("kill -0 {pid} && echo alive"), false).await;
    assert!(!bare.success, "{}", bare.summary);
}

/// A single-quoted echo and `||` alone work too.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn other_literal_echo_tails_are_taken() {
    let registry = BgRegistry::new();
    let pid = eval_start(&registry, "sleep 60").await;
    for check in [
        format!("kill -0 {pid} && echo 'still up'"),
        format!("kill -0 {pid} || echo down"),
    ] {
        let args = serde_json::json!({ "command": check });
        assert!(signals_only_own_background(&args, &registry, None), "{check}");
    }
    let up = eval_bash(&registry, &format!("kill -0 {pid} && echo 'still up'"), false).await;
    assert!(up.success && up.summary.contains("still up"), "{}", up.summary);
    registry.kill(pid);
}

/// Anything more than a literal echo, or a process Koda didn't start, still
/// goes through the usual checks.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_literal_echoes_on_own_processes_are_taken() {
    let registry = BgRegistry::new();
    let pid = eval_start(&registry, "sleep 60").await;
    for not_ours in [
        format!("kill -0 {pid} && rm -rf build"),
        format!("kill -0 {pid} && echo $(rm -rf build)"),
        format!("kill -0 {pid} && echo `rm -rf build`"),
        format!("kill -0 {pid} && echo ok > log"),
        format!("kill -0 {pid} && echo ok; rm -rf build"),
        format!("kill -0 {pid} | echo ok"),
        format!("echo hi && kill {pid}"),
        "kill -0 1 && echo alive".to_string(),
    ] {
        let args = serde_json::json!({ "command": not_ours });
        assert!(
            !signals_only_own_background(&args, &registry, None),
            "{not_ours}"
        );
    }
    registry.kill(pid);
}
