use std::{collections::HashMap, sync::Arc};

use crate::agent::langs::{cmdlang};


#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionResult {
    pub name: String,
    pub result: String,
}

type Function = Arc<dyn Fn(&String, &[String]) -> String + Send + Sync>;

#[derive(Clone)]
pub struct Functions {
    functions: HashMap<String, Function>,
}

impl Functions {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn parse(&self, call: &str) -> Result<FunctionCall, String> {
        let cmd = cmdlang::Cmd::parse(call)?;

        Ok(FunctionCall {
            name: cmd.fname,
            args: cmd.args,
        })
    }

    pub fn register(&mut self, name: impl Into<String>, f: Function) {
        self.functions.insert(name.into(), f);
    }

    pub async fn exec(&self, fc: FunctionCall) -> FunctionResult {
        if let Some(h) = self.functions.get(&fc.name) {

            let res = (h)(&fc.name, &fc.args);

            return FunctionResult {
                name: fc.name,
                result: res,
            };
        };

        return FunctionResult {
            name: fc.name,
            result: "this function does not exist".to_string(),
        };
    }
}
