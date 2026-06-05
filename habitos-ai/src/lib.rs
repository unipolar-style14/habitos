use async_trait::async_trait;

pub mod anthropic;
pub mod ollama;
pub mod openai_compatible;
pub mod prompts;

pub use anthropic::AnthropicClient;
pub use ollama::OllamaClient;
pub use openai_compatible::OpenAiCompatibleClient;
pub use prompts::PromptLoader;

/// Single trait every AI backend implements. Call sites depend on the trait,
/// never on a concrete client.
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, LlmError>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError>;
    async fn ping(&self) -> Result<(), LlmError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("backend not configured — set [ai] in config.toml")]
    NotConfigured,
    #[error("backend unreachable: {0}")]
    Unreachable(String),
    #[error("backend error: {0}")]
    Backend(String),
}

#[derive(Debug, Clone)]
pub struct AiBackendConfig {
    /// "ollama" or "openai-compatible"
    pub kind: String,
    pub model: String,
    pub endpoint: String,
    pub timeout_secs: u64,
    pub api_key: Option<String>,
}

/// Construct a client from a config block. Returns `NotConfigured` if any
/// required field is missing — that's the signal for the CLI to fall back to
/// deterministic output.
pub fn build_client(cfg: &AiBackendConfig) -> Result<Box<dyn LlmClient>, LlmError> {
    match cfg.kind.as_str() {
        "ollama" => Ok(Box::new(OllamaClient::new(
            &cfg.endpoint,
            &cfg.model,
            cfg.timeout_secs,
        ))),
        "openai-compatible" => Ok(Box::new(OpenAiCompatibleClient::new(
            &cfg.endpoint,
            &cfg.model,
            cfg.timeout_secs,
            cfg.api_key.clone(),
        ))),
        "anthropic" => {
            let api_key = cfg.api_key.clone().ok_or_else(|| {
                LlmError::Backend("`anthropic` backend requires `api_key` in [ai] config".into())
            })?;
            Ok(Box::new(AnthropicClient::new(
                &cfg.endpoint,
                &cfg.model,
                cfg.timeout_secs,
                api_key,
            )))
        }
        other => Err(LlmError::Backend(format!(
            "unknown backend `{other}` (expected `ollama`, `openai-compatible`, or `anthropic`)"
        ))),
    }
}
