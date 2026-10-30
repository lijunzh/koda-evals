use koda_core::compact::{self, record_compact_failure, reset_compact_failures};
use koda_core::config::ModelSettings;
use koda_core::persistence::Role;
use koda_core::providers::stream_collector::SseCollector;
use koda_core::providers::TokenUsage;

const WINDOW: usize = 8_000;
const QUESTION: &str = "now answer the real question";

// The failure counter is process-global, so these tests take turns.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn lock_compact_state() -> tokio::sync::MutexGuard<'static, ()> {
    let guard = SERIAL.lock().await;
    reset_compact_failures();
    guard
}

/// A reasoning model that spends every non-streamed call thinking: it stops
/// at the token cap with no text, however many times it is asked. Streamed
/// calls, which carry the real requests, are answered normally.
struct ThinksThroughEverySummary {
    streamed: MockProvider,
}

impl ThinksThroughEverySummary {
    fn new() -> Self {
        Self {
            streamed: MockProvider::new(vec![MockResponse::Text("the real answer".into())]),
        }
    }

    /// The last request that went to the model.
    fn last_request(&self) -> Vec<ChatMessage> {
        self.streamed
            .recorded_calls()
            .last()
            .expect("a request reached the model")
            .clone()
    }
}

#[async_trait]
impl LlmProvider for ThinksThroughEverySummary {
    async fn chat(
        &self,
        _: &[ChatMessage],
        _: &[ToolDefinition],
        _: &ModelSettings,
    ) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: None,
            tool_calls: vec![],
            reasoning: Some("thinking".into()),
            usage: TokenUsage {
                stop_reason: "max_tokens".into(),
                ..Default::default()
            },
        })
    }

    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        settings: &ModelSettings,
    ) -> Result<SseCollector> {
        self.streamed.chat_stream(messages, tools, settings).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }

    fn provider_name(&self) -> &str {
        "reasoning"
    }
}

/// About 42,000 chars of history, 150% of the window, then a user question.
async fn env_past_the_window() -> Env {
    let env = Env::builder().max_context_tokens(WINDOW).build().await;
    let padding = "x".repeat(3000);
    for i in 0..7 {
        env.insert_message(&Role::User, &format!("user {i}: {padding}"))
            .await;
        env.insert_message(&Role::Assistant, &format!("reply {i}: {padding}"))
            .await;
    }
    env.insert_user_message(QUESTION).await;
    env
}

fn assert_fits_the_window(request: &[ChatMessage]) {
    let chars: usize = request
        .iter()
        .filter_map(|m| m.content.as_deref())
        .map(str::len)
        .sum();
    assert!(
        chars < WINDOW * 4,
        "request carries {chars} chars, past a {WINDOW}-token window"
    );
    assert!(
        request
            .iter()
            .filter_map(|m| m.content.as_deref())
            .any(|c| c.contains(QUESTION)),
        "the latest user message must still be sent"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cut_off_reply_with_no_text_does_not_become_the_summary() {
    let _serial = lock_compact_state().await;
    let env = Env::builder().max_context_tokens(100_000).build().await;
    for i in 0..10 {
        env.insert_message(&Role::User, &format!("question {i}"))
            .await;
        env.insert_message(&Role::Assistant, &format!("answer {i}"))
            .await;
    }
    let provider = ThinksThroughEverySummary::new();

    // Reporting a failure and leaving a placeholder are both fine.
    let _ = compact::compact_session_with_provider(
        &env.db,
        &env.session_id,
        env.config.max_context_tokens,
        &env.config.model_settings,
        &provider,
    )
    .await;

    let history = env.db.load_context(&env.session_id).await.unwrap();
    for message in &history {
        let text = message.content.as_deref().unwrap_or("");
        if let Some(summary) = text.strip_prefix("[Compacted conversation summary]") {
            assert!(!summary.trim().is_empty(), "empty summary stored: {text:?}");
        }
    }
    assert!(
        history
            .iter()
            .any(|m| m.content.as_deref() == Some("answer 9")),
        "the recent tail stays"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_summary_still_leaves_the_request_under_the_window() {
    let _serial = lock_compact_state().await;
    let env = env_past_the_window().await;
    let provider = ThinksThroughEverySummary::new();

    let (result, _) = env.run_inference_result(&provider).await;

    assert!(result.is_ok(), "{:?}", result.err());
    assert_fits_the_window(&provider.last_request());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repeated_failures_still_leave_the_request_under_the_window() {
    let _serial = lock_compact_state().await;
    let env = env_past_the_window().await;
    for _ in 0..3 {
        record_compact_failure();
    }
    let provider = ThinksThroughEverySummary::new();

    let (result, _) = env.run_inference_result(&provider).await;

    assert!(result.is_ok(), "{:?}", result.err());
    assert_fits_the_window(&provider.last_request());
}
