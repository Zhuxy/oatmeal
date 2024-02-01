pub mod langchain;
pub mod ollama;
pub mod openai;
pub mod azureai;
use anyhow::bail;
use anyhow::Result;

use crate::domain::models::BackendBox;
use crate::domain::models::BackendName;

pub struct BackendManager {}

impl BackendManager {
    pub fn get(name: BackendName) -> Result<BackendBox> {
        if name == BackendName::LangChain {
            return Ok(Box::<langchain::LangChain>::default());
        }

        if name == BackendName::Ollama {
            return Ok(Box::<ollama::Ollama>::default());
        }

        if name == BackendName::OpenAI {
            return Ok(Box::<openai::OpenAI>::default());
        }

        if name == BackendName::AzureAI {
            return Ok(Box::<azureai::AzureAI>::default());
        }
        bail!(format!("No backend implemented for {name}"))
    }
}
