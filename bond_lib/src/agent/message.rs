use std::collections::HashMap;

use crate::agent::function::{FunctionCall, FunctionResult};

#[derive(Debug, Clone)]
pub enum MessageAuthor {
    LLM,
    Function,
    User,
    System,
}

#[derive(Debug, Clone)]
pub enum MessageValue {
    Text(String),
    TextLLM(String, String, String),
    FunctionCall(FunctionCall),
    FunctionResult(FunctionResult),
    Error(String),
}

impl MessageValue {
    pub fn as_str(&self) -> String {
        match self {
            MessageValue::Text(x) => x.to_string(),
            MessageValue::FunctionCall(x) => format!("func call: {}\n{:?}", x.name, x.args),
            MessageValue::FunctionResult(x) => format!("{} result: {}", x.name, x.result),
            MessageValue::Error(x) => format!("error: {}", x),
            MessageValue::TextLLM(x, _, _) => x.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageExtra {
    None,
    Map(HashMap<String, String>),
}



#[derive(Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub time: u64,
    pub author: MessageAuthor,
    pub value: MessageValue,
    pub extra: MessageExtra,
}