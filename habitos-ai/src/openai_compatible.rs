use crate::{LlmClient, LlmError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct OpenAiCompatibleClient {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

impl OpenAiCompatibleClient {
    pub fn new(endpoint: &str, model: &str, timeout_secs: u64, api_key: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("reqwest client build with timeout cannot fail");
        Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(k) => req.bearer_auth(k),
            None => req,
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
    async fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        let url = format!("{}/v1/chat/completions", self.endpoint);
        let req = ChatRequest {
            model: &self.model,
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
        let body: ChatResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::Backend(e.to_string()))?;
        body.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| LlmError::Backend("empty choices".into()))
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        let url = format!("{}/v1/embeddings", self.endpoint);
        let req = EmbedRequest {
            model: &self.model,
            input: text,
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
        let body: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::Backend(e.to_string()))?;
        body.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| LlmError::Backend("empty data".into()))
    }

    async fn ping(&self) -> Result<(), LlmError> {
        let url = format!("{}/v1/models", self.endpoint);
        let resp = self
            .auth(self.client.get(&url))
            .send()
            .await
            .map_err(|e| LlmError::Unreachable(e.to_string()))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(LlmError::Backend(format!("HTTP {}", resp.status())))
        }
    }
}
