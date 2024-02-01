#[cfg(test)]
#[path = "azureai_test.rs"]
mod tests;


use anyhow::bail;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tokio_util::io::StreamReader;

use crate::configuration::Config;
use crate::configuration::ConfigKey;
use crate::domain::models::Author;
use crate::domain::models::Backend;
use crate::domain::models::BackendName;
use crate::domain::models::BackendPrompt;
use crate::domain::models::BackendResponse;
use crate::domain::models::Event;

fn convert_err(err: reqwest::Error) -> std::io::Error {
    let err_msg = err.to_string();
    return std::io::Error::new(std::io::ErrorKind::Interrupted, err_msg);
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MessageRequest {
    role: String,
    content: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionRequest {
    model: String,
    messages: Vec<MessageRequest>,
    stream: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionDeltaResponse {
    content: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionChoiceResponse {
    delta: CompletionDeltaResponse,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoiceResponse>,
}

pub struct AzureAI {
    url: String,
    api_key: String,
    api_version: String,
    deployment_id: String,
}

impl Default for AzureAI {
    fn default() -> Self {
        return AzureAI {
            url: Config::get(ConfigKey::AzureAIURL),
            api_key: Config::get(ConfigKey::AzureAIAPIKey),
            api_version: Config::get(ConfigKey::AzureAIAPIVersion),
            deployment_id: Config::get(ConfigKey::AzureAIDeploymentID),
        };
    }
}

#[async_trait]
impl Backend for AzureAI {
    fn name(&self) -> BackendName {
        return BackendName::AzureAI;
    }

    #[allow(clippy::implicit_return)]
    async fn health_check(&self) -> Result<()> {
        if self.url.is_empty() {
            bail!("AzureAI URL is not defined");
        }
        if self.api_key.is_empty() {
            bail!("AzureAI token is not defined");
        }

        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn list_models(&self) -> Result<Vec<String>> {
        return Ok(vec![String::from("gpt-35-turbo")]);
    }

    #[allow(clippy::implicit_return)]
    async fn get_completion<'a>(
        &self,
        prompt: BackendPrompt,
        tx: &'a mpsc::UnboundedSender<Event>,
    ) -> Result<()> {
        let mut messages: Vec<MessageRequest> = vec![];
        if !prompt.backend_context.is_empty() {
            messages = serde_json::from_str(&prompt.backend_context)?;
        }
        messages.push(MessageRequest {
            role: "user".to_string(),
            content: prompt.text,
        });

        let req = CompletionRequest {
            model: Config::get(ConfigKey::Model),
            messages: messages.clone(),
            stream: true,
        };

        let res = reqwest::Client::new()
            .post(
                format!("{url}/openai/deployments/{deploymentId}/chat/completions?api-version={apiVersion}", 
                    url = self.url,
                    deploymentId = self.deployment_id,
                    apiVersion = self.api_version))
            .header("api-key", self.api_key.to_string())
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            tracing::error!(
                status = res.status().as_u16(),
                "Failed to make completion request to Azure AI"
            );
            bail!("Failed to make completion request to Azure AI");
        }

        let stream = res.bytes_stream().map_err(convert_err);
        let mut lines_reader = StreamReader::new(stream).lines();

        let mut last_message = "".to_string();
        while let Ok(line) = lines_reader.next_line().await {
            if line.is_none() {
                break;
            }

            let mut cleaned_line = line.unwrap().trim().to_string();
            if cleaned_line.ends_with("[DONE]") {
                break;
            }
            if cleaned_line.starts_with("data:") {
                cleaned_line = cleaned_line.split_off(5).trim().to_string();
            }
            if cleaned_line.is_empty() {
                continue;
            }


            let ores: CompletionResponse = serde_json::from_str(&cleaned_line).unwrap();
            tracing::debug!(body = ?ores, "Completion response");

            if let Some(text) = &ores.choices[0].delta.content {
                last_message += text;
                let msg = BackendResponse {
                    author: Author::Model,
                    text: text.to_string(),
                    done: false,
                    context: None,
                };

                tx.send(Event::BackendPromptResponse(msg))?;
            };
        }

        messages.push(MessageRequest {
            role: "assistant".to_string(),
            content: last_message.to_string(),
        });

        let msg = BackendResponse {
            author: Author::Model,
            text: "".to_string(),
            done: true,
            context: Some(serde_json::to_string(&messages)?),
        };
        tx.send(Event::BackendPromptResponse(msg))?;

        return Ok(());
    }
}
