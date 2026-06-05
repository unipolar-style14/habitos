use crate::{LlmClient, LlmError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 2048;

pub struct AnthropicClient {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    api_key: String,
    max_tokens: u32,
}

impl AnthropicClient {
    pub fn new(endpoint: &str, model: &str, timeout_secs: u64, api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("reqwest client build with timeout cannot fail");
        Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
    }
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        let url = format!("{}/v1/messages", self.endpoint);
        let req = MessagesRequest {
            model: &self.model,
            max_tokens: self.max_tokens,
            messages: vec![Message {
                role: "user",
                content: prompt,
            }],
        };
        let resp = self
            .auth(self.client.post(&url).json(&req))
            .send()
            .await
            .map_err(|e| LlmError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Backend(format!("HTTP {status}: {body}")));
        }
        let body: MessagesResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::Backend(e.to_string()))?;
        body.content
            .into_iter()
            .find_map(|c| match c {
                ContentBlock::Text { text } => Some(text),
                ContentBlock::Other => None,
            })
            .ok_or_else(|| LlmError::Backend("no text content in response".into()))
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, LlmError> {
        // Anthropic's API does not provide embeddings. Users who want
        // `habitos ask` should run Ollama in parallel with an embedding
        // model. A future config option will allow splitting completions and
        // embeddings across two backends.
        Err(LlmError::Backend(
            "Anthropic API does not provide embeddings; run Ollama (or another \
             embedding backend) in parallel for `habitos ask`."
                .into(),
        ))
    }

    async fn ping(&self) -> Result<(), LlmError> {
        // No cheap probe endpoint; do a 1-token completion as a liveness check.
        let url = format!("{}/v1/messages", self.endpoint);
        let req = MessagesRequest {
            model: &self.model,
            max_tokens: 1,
            messages: vec![Message {
                role: "user",
                content: "hi",
            }],
        };
        let resp = self
            .auth(self.client.post(&url).json(&req))
            .send()
            .await
            .map_err(|e| LlmError::Unreachable(e.to_string()))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(LlmError::Backend(format!("HTTP {status}: {body}")))
        }
    }
}
