use anyhow::bail;
use anyhow::Result;
use test_utils::insta_snapshot;
use tokio::sync::mpsc;

use super::CompletionChoiceResponse;
use super::CompletionDeltaResponse;
use super::CompletionResponse;
use super::MessageRequest;
use super::AzureAI;
use crate::domain::models::Author;
use crate::domain::models::Backend;
use crate::domain::models::BackendPrompt;
use crate::domain::models::BackendResponse;
use crate::domain::models::Event;
use crate::infrastructure::backends::azureai::CompletionRequest;

fn to_res(action: Option<Event>) -> Result<BackendResponse> {
    let act = match action.unwrap() {
        Event::BackendPromptResponse(res) => res,
        _ => bail!("Wrong type from recv"),
    };

    return Ok(act);
}

#[tokio::test]
async fn it_gets_completions() -> Result<()> {
    let first_line = serde_json::to_string(&CompletionResponse {
        choices: vec![CompletionChoiceResponse {
            delta: CompletionDeltaResponse {
                content: Some("Hello ".to_string()),
            },
        }],
    })?;

    let second_line = serde_json::to_string(&CompletionResponse {
        choices: vec![CompletionChoiceResponse {
            delta: CompletionDeltaResponse {
                content: Some("World".to_string()),
            },
        }],
    })?;

    let body = [first_line, second_line].join("\n");
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![MessageRequest {
            role: "assistant".to_string(),
            content: "How may I help you?".to_string(),
        }])?,
    };

    // let mut server = mockito::Server::new();
    // let mock = server
    //     .mock("POST", "/v1/chat/completions")
    //     .match_header("Authorization", "Bearer abc")
    //     .with_status(200)
    //     .with_body(body)
    //     .create();

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let backend = AzureAI {
        url: std::env::var("OATMEAL_AZUREAI_URL").unwrap(),
        api_key: std::env::var("OATMEAL_AZUREAI_API_KEY").unwrap(),
        api_version: std::env::var("OATMEAL_AZUREAI_API_VERSION").unwrap(),
        deployment_id: std::env::var("OATMEAL_AZUREAI_DEPLOYMENT_ID").unwrap(),
    };

    println!("{:?}", backend.url);
    println!("{:?}", backend.api_key);
    println!("{:?}", backend.api_version);
    println!("{:?}", backend.deployment_id);

    backend.get_completion(prompt, &tx).await?;

    // mock.assert();

    let first_recv = to_res(rx.recv().await)?;
    let second_recv = to_res(rx.recv().await)?;
    let third_recv = to_res(rx.recv().await)?;

    println!("{:?}", first_recv.text);
    println!("{:?}", second_recv.text);
    println!("{:?}", third_recv.text);

    return Ok(());
}

#[tokio::test]
async fn azure_test() -> Result<()> {
    let url = std::env::var("OATMEAL_AZUREAI_URL").unwrap();
    let api_key = std::env::var("OATMEAL_AZUREAI_API_KEY").unwrap();
    let api_version = std::env::var("OATMEAL_AZUREAI_API_VERSION").unwrap();
    let deployment_id = std::env::var("OATMEAL_AZUREAI_DEPLOYMENT_ID").unwrap();

    let url = format!("{url}/openai/deployments/{deploymentId}/chat/completions?api-version={apiVersion}", 
                url = url,
                deploymentId = deployment_id,
                apiVersion = api_version);

    println!("{:?}", url);


    let messages: Vec<MessageRequest> = vec![
        MessageRequest {
            role: "user".to_string(),
            content: "hello".to_string(),
        }];

    let req = CompletionRequest {
        model: "gpt-35-turbo".to_string(),
        messages: messages.clone(),
        stream: true,
    };

    let res = reqwest::Client::new()
        .post(url)
        .header("api-key", api_key.to_string())
        .json(&req)
        .send()
    .await.unwrap();

    println!("{:?}", res.text().await.unwrap());

    return Ok(());
}
