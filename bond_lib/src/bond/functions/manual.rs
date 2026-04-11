use std::collections::HashMap;

use crate::agent::function::{BoxFuture, Function};

pub struct Manual {
    manuals: HashMap<String, String>,
}

impl Manual {
    pub fn new() -> Self {
        Self { manuals: HashMap::new() }
    }

    pub fn register(&mut self, ns: &str, manual: &str) {
        self.manuals.insert(ns.to_string(), manual.to_string());
    }
}

impl Function for Manual {
    fn call(&self, x: crate::taglang::Tag) -> BoxFuture {
        let ns = x.get("namespace").unwrap().as_str();
        let manual = self.manuals.get(ns).unwrap_or(&"no such namespace".to_string()).to_string();

        Box::pin(async move {
            manual
        })
    }
}