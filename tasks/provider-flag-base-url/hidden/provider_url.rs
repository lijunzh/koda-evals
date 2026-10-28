#[test]
fn provider_alone_moves_to_that_providers_default_url() {
    let config = KodaConfig::default_for_testing(ProviderType::LMStudio)
        .with_overrides(None, None, Some("anthropic".into()));
    assert_eq!(config.provider_type, ProviderType::Anthropic);
    assert_eq!(config.base_url, "https://api.anthropic.com");

    let config = KodaConfig::default_for_testing(ProviderType::OpenAI)
        .with_overrides(None, None, Some("gemini".into()));
    assert_eq!(config.provider_type, ProviderType::Gemini);
    assert_eq!(config.base_url, "https://generativelanguage.googleapis.com");
}

#[test]
fn custom_url_for_the_same_provider_is_kept() {
    let mut config = KodaConfig::default_for_testing(ProviderType::Ollama);
    config.base_url = "http://127.0.0.1:11434/v1".to_string();
    let config = config.with_overrides(None, None, Some("ollama".into()));
    assert_eq!(config.provider_type, ProviderType::Ollama);
    assert_eq!(config.base_url, "http://127.0.0.1:11434/v1");
}

#[test]
fn explicit_url_still_wins() {
    let config = KodaConfig::default_for_testing(ProviderType::LMStudio).with_overrides(
        Some("https://my-proxy.example".into()),
        None,
        Some("anthropic".into()),
    );
    assert_eq!(config.provider_type, ProviderType::Anthropic);
    assert_eq!(config.base_url, "https://my-proxy.example");
}
