use serde_json::json;

use crate::{
    agent::{
        function::Functions,
        message::{Message, MessageAuthor, MessageExtra, MessageValue},
    },
};

#[derive(Clone)]
pub struct OAILLM {
    model: String,
    api_base: String,
    api_key: String,
}

pub fn translate_author(x: &MessageAuthor) -> &'static str {
    match x {
        MessageAuthor::LLM => "assistant",
        MessageAuthor::Function => todo!(),
        MessageAuthor::User => "user",
        MessageAuthor::System => "system",
    }
}

pub fn translate_message(x: &Message) -> serde_json::Value {
    match x.author {
        MessageAuthor::LLM => {
            if let MessageValue::TextLLM(_, id, status) = &x.value {
                serde_json::json!({"type": "message", "role": translate_author(&x.author), "id": id, "status": status, "content": [{"type": "output_text", "text": x.value.as_str()}]})
            } else {
                todo!()
            }
        }
        MessageAuthor::User | MessageAuthor::System => {
            serde_json::json!({"role": translate_author(&x.author), "content": [{"type": "input_text", "text": x.value.as_str()}]})
        }
        MessageAuthor::Function => todo!(),
    }
}

impl OAILLM {
    pub async fn new(model: &str, api_base: &str, api_key: &str) -> Self {
        Self {
            model: model.to_string(),
            api_base: api_base.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub fn info(&self) -> String {
        format!("OpenAI interface | Model: {}", self.model)
    }

    pub async fn send(&self, messages: &[Message], functions: &Functions) -> Vec<Message> {
        println!("sending {} messages to llm", messages.len());
        let last_message_time = messages.last().unwrap().id;

        let messages_payload: serde_json::Value = serde_json::json!(
            messages
                .iter()
                .map(|x| translate_message(x))
                .collect::<Vec<serde_json::Value>>()
        );

        let payload = serde_json::json!({
            "model": self.model,
            "input": messages_payload,
            "reasoning": {"effort": "low"},
            "tools": [
            {
                "type": "function",
                "name": "gateway",
                "description": "This is a gateway function to call other functions.",
                "parameters": {
                "type": "object",
                "properties": {
                    "x": {
                    "type": "string",
                    "description": "This is a string that will be parsed using taglang into function calls."
                    },
                },
                "required": ["x"]
                }
            }
            ],
            "tool_choice": "auto"
        });

        println!("{}", payload.to_string());

        let c = reqwest::Client::new();

        let resp = c
            .post(&self.api_base)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&payload).unwrap())
            .send()
            .await
            .expect("Network request failed");

        let resp_data = resp.text().await.expect("Failed to read response body");
        println!("==========> {:?}", resp_data);

        let parsed: serde_json::Value = match serde_json::from_str(&resp_data) {
            Ok(json) => json,
            Err(_) => {
                return vec![Message {
                    id: 0,
                    time: 0,
                    author: MessageAuthor::System,
                    value: MessageValue::Error(format!(
                        "OpenAI returned garbage/non-JSON: {}",
                        resp_data
                    )),

                    extra: MessageExtra::None,
                }];
            }
        };

        if let Some(error) = parsed.get("error") {
            if !error.is_null() {
                return vec![Message {
                    id: 0,
                    time: 0,
                    author: MessageAuthor::System,
                    value: MessageValue::Error(format!(
                        "OpenAI returned garbage/non-JSON: {}",
                        resp_data
                    )),
                extra: MessageExtra::None,
                }];
            }
        }

        let mut result_messages = Vec::new();

        if let Some(outputs) = parsed.get("output").and_then(|o| o.as_array()) {
            for item in outputs {
                let message_type = item.get("type").and_then(|t| t.as_str()).unwrap();

                if message_type == "message" {
                    let role = item
                        .get("role")
                        .and_then(|r| r.as_str())
                        .unwrap_or("assistant");

                    let id = item
                        .get("id")
                        .and_then(|r| r.as_str())
                        .unwrap_or("")
                        .to_string();

                    let status = item
                        .get("status")
                        .and_then(|r| r.as_str())
                        .unwrap_or("")
                        .to_string();

                    let author = match role {
                        "assistant" => MessageAuthor::LLM,
                        "user" => MessageAuthor::User,
                        "system" => MessageAuthor::System,
                        _ => MessageAuthor::LLM,
                    };

                    let mut combined_text = String::new();

                    if let Some(contents) = item.get("content").and_then(|c| c.as_array()) {
                        for content_block in contents {
                            if content_block.get("type").and_then(|t| t.as_str())
                                == Some("output_text")
                            {
                                if let Some(text) =
                                    content_block.get("text").and_then(|t| t.as_str())
                                {
                                    combined_text.push_str(text);
                                }
                            }
                        }
                    }

                    result_messages.push(Message {
                        id: last_message_time,
                        time: last_message_time,
                        author: author,
                        value: crate::agent::message::MessageValue::TextLLM(
                            combined_text,
                            id,
                            status,
                        ),
                extra: MessageExtra::None,
                    });
                } else if message_type == "reasoning" {
                } else if message_type == "function_call" {
                    let func_name = item.get("name").and_then(|r| r.as_str()).unwrap();
                    let x = item.get("arguments").and_then(|r| r.as_str()).unwrap();
                    let x = serde_json::from_str::<serde_json::Value>(x).unwrap();
                    let x = x.get("x").unwrap().as_str().unwrap();

                    if func_name == "gateway" {
                        let fcall = functions.parse(x);
                        if fcall.is_ok() {
                            result_messages.push(Message {
                                id: last_message_time,
                                time: last_message_time,
                                author: MessageAuthor::System,
                                value: MessageValue::FunctionCall(fcall.unwrap()),
                extra: MessageExtra::None,
                            });
                        } else {
                            result_messages.push(Message {
                                id: last_message_time,
                                time: last_message_time,
                                author: MessageAuthor::System,
                                value: MessageValue::Error(fcall.unwrap_err()),
                extra: MessageExtra::None,
                            });
                        }
                    } else {
                        result_messages.push(Message {
                            id: last_message_time,
                            time: last_message_time,
                            author: MessageAuthor::System,
                            value: MessageValue::Error("Error: Invalid function call, you can only call functions through gateway.".to_string()),
                extra: MessageExtra::None,
                        });
                    }
                } else {
                    println!("Warning: Unknown message type {}", message_type);
                }
            }
        } else {
            println!("Warning: No 'output' array found in response JSON");
        }

        result_messages
    }
}