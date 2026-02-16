use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::history::ConversationHistory;
use super::protocol::{self, ShipCommand};

const API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent";
const AI_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TOKENS: i32 = 256;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiRequest {
    system_instruction: GeminiContent,
    contents: Vec<GeminiContent>,
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    max_output_tokens: i32,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<GeminiPart>,
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

        let contents: Vec<GeminiContent> = self
            .history
            .messages()
            .iter()
            .map(|m| GeminiContent {
                role: Some(m.role.clone()),
                parts: vec![GeminiPart {
                    text: m.content.clone(),
                }],
            })
            .collect();

        let request = ApiRequest {
            system_instruction: GeminiContent {
                role: None,
                parts: vec![GeminiPart {
                    text: self.system_prompt.clone(),
                }],
            },
            contents,
            generation_config: GenerationConfig {
                max_output_tokens: MAX_TOKENS,
            },
        };

        let url = format!("{}?key={}", API_URL, self.api_key);

        let result = tokio::time::timeout(AI_TIMEOUT, async {
            self.client
                .post(&url)
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await
        })
        .await;

        match result {
            Ok(Ok(response)) => {
                if let Ok(api_response) = response.json::<ApiResponse>().await {
                    if let Some(text) = api_response
                        .candidates
                        .first()
                        .and_then(|c| c.content.parts.first())
                        .map(|p| &p.text)
                    {
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
