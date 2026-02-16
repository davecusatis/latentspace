use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::history::ConversationHistory;
use super::protocol::{self, ShipCommand};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-5-20250929";
const AI_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TOKENS: i32 = 256;

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: i32,
    system: String,
    messages: Vec<super::history::Message>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

pub struct AiAgent {
    client: Client,
    api_key: String,
    system_prompt: String,
    pub history: ConversationHistory,
}

impl AiAgent {
    pub fn new(api_key: String, system_prompt: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            system_prompt,
            history: ConversationHistory::new(),
        }
    }

    /// Send the game state to the AI and get a command back.
    /// Returns the default (drift) command on timeout or error.
    pub async fn get_command(&mut self, game_state_json: &str) -> ShipCommand {
        self.history.add_user(game_state_json.to_string());

        let request = ApiRequest {
            model: MODEL.to_string(),
            max_tokens: MAX_TOKENS,
            system: self.system_prompt.clone(),
            messages: self.history.messages().to_vec(),
        };

        let result = tokio::time::timeout(AI_TIMEOUT, async {
            self.client
                .post(API_URL)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await
        })
        .await;

        match result {
            Ok(Ok(response)) => {
                if let Ok(api_response) = response.json::<ApiResponse>().await {
                    if let Some(text) = api_response.content.first().and_then(|c| c.text.as_ref()) {
                        self.history.add_assistant(text.clone());
                        match protocol::parse_command(text) {
                            Ok(cmd) => return cmd,
                            Err(_) => return ShipCommand::default(),
                        }
                    }
                }
                ShipCommand::default()
            }
            _ => ShipCommand::default(),
        }
    }
}
