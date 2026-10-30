use koda_core::config::ProviderType;
use koda_core::last_provider::save_last_provider;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// A local endpoint that answers every request with 404 and counts them.
/// Startup probes the model's capabilities on the endpoint koda chose, so
/// the count shows which provider the session opened on.
struct Endpoint {
    url: String,
    hits: Arc<AtomicUsize>,
}

fn endpoint() -> Endpoint {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let hits = Arc::new(AtomicUsize::new(0));
    let counted = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            counted.fetch_add(1, Ordering::SeqCst);
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            let _ = stream
                .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    });
    Endpoint { url, hits }
}

/// A config dir whose database remembers LM Studio at `saved_url`.
async fn config_home_remembering(saved_url: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let db = koda_core::db::Database::init(&home.path().join("koda"))
        .await
        .unwrap();
    save_last_provider(
        &db,
        &ProviderType::LMStudio.to_string(),
        saved_url,
        "saved-local-model",
    )
    .await
    .unwrap();
    home
}

/// Start interactive koda and wait until it has contacted one of `endpoints`.
fn open_tui(home: &std::path::Path, args: &[&str], endpoints: &[&Endpoint]) {
    let project = tempfile::tempdir().unwrap();
    let mut child = Command::new(koda_bin())
        .args(args)
        .arg("--project-root")
        .arg(project.path())
        .env("XDG_CONFIG_HOME", home)
        .env("ANTHROPIC_API_KEY", "test-key")
        .env_remove("KODA_PROVIDER")
        .env_remove("KODA_MODEL")
        .env_remove("KODA_BASE_URL")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline
        && endpoints.iter().all(|e| e.hits.load(Ordering::SeqCst) == 0)
        && child.try_wait().unwrap().is_none()
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_provider_beats_the_saved_one() {
    let saved = endpoint();
    let chosen = endpoint();
    let home = config_home_remembering(&format!("{}/v1", saved.url)).await;

    open_tui(
        home.path(),
        &[
            "--provider",
            "anthropic",
            "--base-url",
            &chosen.url,
            "--model",
            "claude-sonnet-5-5",
        ],
        &[&saved, &chosen],
    );

    assert_eq!(
        saved.hits.load(Ordering::SeqCst),
        0,
        "opened on the saved local provider"
    );
    assert!(
        chosen.hits.load(Ordering::SeqCst) > 0,
        "never contacted the provider on the command line"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_flags_restores_the_saved_provider() {
    let saved = endpoint();
    let home = config_home_remembering(&format!("{}/v1", saved.url)).await;

    open_tui(home.path(), &[], &[&saved]);

    assert!(
        saved.hits.load(Ordering::SeqCst) > 0,
        "the last-used provider was not restored"
    );
}
