use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Provider {
    pub name: String,
    pub backend: String,
    pub api_base: String,
    pub api_key: String,

    #[serde(default)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(rename="provider")]
    pub providers: Vec<Provider>,
    pub model: String,
    #[serde(rename="provider_name")]
    pub provider: String,
    pub blocked_functions: Vec<String>,

    #[serde(default)]
    pub extra: HashMap<String, String>,
}

