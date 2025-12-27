use crate::message::{Message, Role};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};

const DEFAULT_MODEL: &str = "gpt-5-mini";
const API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Clone)]
pub struct OpenAIClient {
    api_key: String,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn send_message_streaming<F>(
        &self,
        messages: &[Message],
        mut on_token: F,
    ) -> Result<(), String>
    where
        F: FnMut(String),
    {
        let openai_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|msg| OpenAIMessage {
                role: match msg.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                },
                content: msg.content.clone(),
            })
            .collect();

        let request_body = OpenAIRequest {
            model: DEFAULT_MODEL.to_string(),
            messages: openai_messages,
            stream: true,
        };

        let mut response = ureq::post(API_URL)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(&request_body)
            .map_err(|e| format!("API request failed: {}", e))?;

        let reader = BufReader::new(response.body_mut().as_reader());

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read response: {}", e))?;

            if line.is_empty() {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }

                if let Ok(chunk) = serde_json::from_str::<StreamResponse>(data) {
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            on_token(content.clone());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
