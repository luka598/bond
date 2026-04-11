use std::collections::HashMap;

use crate::agent::{
    function::Functions,
    message::{Message, MessageAuthor, MessageExtra, MessageValue},
};

const TOOL_CALL_ID_KEY: &str = "tool_call_id";
const CACHE_LOOKBACK: usize = 20;
const MAX_CACHE_BREAKPOINTS: usize = 4;


fn insert_cache_marker(msg: &mut serde_json::Value) {
    let cache_control = serde_json::json!({ "type": "ephemeral" });
    match msg.get_mut("content") {
        Some(serde_json::Value::Array(blocks)) => {
            if let Some(last) = blocks.last_mut() {
                last["cache_control"] = cache_control;
            }
        }
        Some(serde_json::Value::String(s)) => {
            let text = s.clone();
            msg["content"] = serde_json::json!([{
                "type": "text",
                "text": text,
                "cache_control": cache_control
            }]);
        }
        _ => {}
    }
}

fn translate_message(msg: &Message) -> Option<serde_json::Value> {
    match msg.author {
        MessageAuthor::System => Some(serde_json::json!({
            "role": "system",
            "content": msg.value.as_str()
        })),
        MessageAuthor::User => Some(serde_json::json!({
            "role": "user",
            "content": msg.value.as_str()
        })),
        MessageAuthor::LLM => Some(serde_json::json!({
            "role": "assistant",
            "content": msg.value.as_str()
        })),
        MessageAuthor::Function => {
            let id = match &msg.extra {
                MessageExtra::Map(m) => m.get(TOOL_CALL_ID_KEY).map(|s| s.as_str()),
                MessageExtra::None => None,
            };
            match id {
                Some(id) => Some(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": id,
                    "content": msg.value.as_str()
                })),
                None => {
                    println!(
                        "Warning: dropping Function message — no '{}' in extra",
                        TOOL_CALL_ID_KEY
                    );
                    None
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct OpenRouterLLM {
    model: String,
    api_key: String,
    api_base: String,
    providers: Vec<String>,
}

impl OpenRouterLLM {
    pub async fn new(model: &str, api_key: &str, providers: &[String]) -> Self {
        Self {
            model: model.to_string(),
            api_key: api_key.to_string(),
            api_base: "https://openrouter.ai/api/v1/chat/completions".to_string(),
            providers: providers.to_vec(),
        }
    }

    pub fn info(&self) -> String {
        format!("OpenRouter interface | Model: {}", self.model)
    }

    pub async fn send(&self, messages: &[Message], functions: &Functions) -> Vec<Message> {
        println!("sending {} messages to openrouter", messages.len());
        let last_message_time = messages.last().unwrap().id;


        let mut payload_messages: Vec<serde_json::Value> = messages
            .iter()
            .filter_map(translate_message)
            .collect();

        let n = payload_messages.len();
        for i in 0..MAX_CACHE_BREAKPOINTS {
            let idx = (n - 1).saturating_sub(i * CACHE_LOOKBACK);
            insert_cache_marker(&mut payload_messages[idx]);
            if idx == 0 {
                break;
            }
        }

        let payload = serde_json::json!({
            "model": &self.model,
            "messages": payload_messages,
            "provider": {
                "order": &self.providers,
                "allow_fallbacks": true,
            },
            "tools": [{
                "type": "function",
                "function": {
                    "name": "gateway",
                    "description": "Gateway function to call other functions.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "x": {
                                "type": "string",
                                "description": "taglang-encoded function call string."
                            }
                        },
                        "required": ["x"]
                    }
                }
            }],
            "tool_choice": "auto"
        });

        println!("{}", payload);

        let client = reqwest::Client::new();
        let resp = client
            .post(&self.api_base)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/luka598/bond")
            .body(serde_json::to_string(&payload).unwrap())
            .send()
            .await
            .expect("Network request failed");

        let resp_data = resp.text().await.expect("Failed to read response body");
        println!("==========> {:?}", resp_data);

        let parsed: serde_json::Value = match serde_json::from_str(&resp_data) {
            Ok(json) => json,
            Err(_) => return error_msg(format!("OpenRouter returned non-JSON: {}", resp_data)),
        };

        if let Some(err) = parsed.get("error") {
            if !err.is_null() {
                return error_msg(format!("OpenRouter error: {}", err));
            }
        }

        let mut result = Vec::new();

        let choice = match parsed
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
        {
            Some(c) => c,
            None => {
                println!("Warning: no choices in OpenRouter response");
                return result;
            }
        };

        let msg = match choice.get("message") {
            Some(m) => m,
            None => {
                println!("Warning: no message in choice");
                return result;
            }
        };


        if let Some(text) = msg.get("content").and_then(|c| c.as_str()) {
            if !text.is_empty() {
                result.push(Message {
                    id: last_message_time,
                    time: last_message_time,
                    author: MessageAuthor::LLM,
                    value: MessageValue::Text(text.to_string()),
                    extra: MessageExtra::None,
                });
            }
        }

        if let Some(tool_calls) = msg.get("tool_calls").and_then(|tc| tc.as_array()) {
            for tc in tool_calls {
                let tool_call_id = tc
                    .get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();

                let func_name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");

                let arguments_str = tc
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|a| a.as_str())
                    .unwrap_or("{}");

                if func_name != "gateway" {
                    result.push(Message {
                        id: last_message_time,
                        time: last_message_time,
                        author: MessageAuthor::System,
                        value: MessageValue::Error(format!(
                            "Invalid function call '{}': only 'gateway' is allowed.",
                            func_name
                        )),
                        extra: MessageExtra::None,
                    });
                    continue;
                }

                let args: serde_json::Value =
                    serde_json::from_str(arguments_str).unwrap_or(serde_json::json!({}));
                let x = args.get("x").and_then(|v| v.as_str()).unwrap_or("");

                let mut extra_map = HashMap::new();
                extra_map.insert(TOOL_CALL_ID_KEY.to_string(), tool_call_id);

                let fcall = functions.parse(x);
                result.push(Message {
                    id: last_message_time,
                    time: last_message_time,
                    author: MessageAuthor::System,
                    value: match fcall {
                        Ok(f) => MessageValue::FunctionCall(f),
                        Err(e) => MessageValue::Error(e),
                    },
                    extra: MessageExtra::Map(extra_map),
                });
            }
        }

        if let Some(usage) = parsed.get("usage") {
            let prompt     = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let cached     = usage.get("prompt_tokens_details")
                                  .and_then(|d| d.get("cached_tokens"))
                                  .and_then(|v| v.as_u64()).unwrap_or(0);
            let completion = usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            println!(
                "tokens — prompt: {} (cached: {}), completion: {}",
                prompt, cached, completion
            );
        }

        result
    }
}

fn error_msg(msg: String) -> Vec<Message> {
    vec![Message {
        id: 0,
        time: 0,
        author: MessageAuthor::System,
        value: MessageValue::Error(msg),
        extra: MessageExtra::None,
    }]
}