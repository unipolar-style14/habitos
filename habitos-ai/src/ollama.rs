use crate::{LlmClient, LlmError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct OllamaClient {
    client: reqwest::Client,
    endpoint: String,
    model: String,
}

impl OllamaClient {
    pub fn new(endpoint: &str, model: &str, timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("reqwest client build with timeout cannot fail");
        Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        let url = format!("{}/api/generate", self.endpoint);
        let req = GenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
        };
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| LlmError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Backend(format!("HTTP {status}: {body}")));
        }
        let body: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::Backend(e.to_string()))?;
        Ok(body.response)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        let url = format!("{}/api/embeddings", self.endpoint);
        let req = EmbedRequest {
            model: &self.model,
            prompt: text,
        };
        let resp = self
            .client
            .post(&url)
            .json(&req)
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
        Ok(body.embedding)
    }

    async fn ping(&self) -> Result<(), LlmError> {
        let url = format!("{}/api/tags", self.endpoint);
        let resp = self
            .client
            .get(&url)
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
