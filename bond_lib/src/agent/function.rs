use std::{collections::HashMap, pin::Pin};

use crate::taglang;

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub args: taglang::Tag,
}

#[derive(Debug, Clone)]
pub struct FunctionResult {
    pub name: String,
    pub result: String,
}

//
// dynamic dispatch
//

pub type BoxFuture = Pin<Box<dyn Future<Output = String> + Send>>;

pub trait Function: Send + Sync + 'static {
    fn call(&self, x: taglang::Tag) -> BoxFuture;
}

impl<F, Fut> Function for F
where
    F: Fn(taglang::Tag) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = String> + Send + 'static,
{
    fn call(&self, x: taglang::Tag) -> BoxFuture {
        Box::pin(self(x))
    }
}

pub struct Functions {
    functions: HashMap<String, Box<dyn Function>>,
}

impl Functions {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn parse(&self, call: &str) -> Result<FunctionCall, String> {
        let root_tag = taglang::parse(call)?;
        let function_name = root_tag
            .get("function")
            .ok_or("no tag function")?
            .get("name")
            .ok_or("no tag function->name")?
            .as_str()
            .to_string();
        let args = root_tag
            .get("function")
            .unwrap()
            .get("args")
            .ok_or("no tag function->args")?
            .clone();

        Ok(FunctionCall {
            name: function_name,
            args: args,
        })
    }

    pub fn register(&mut self, name: impl Into<String>, f: impl Function) {
        self.functions.insert(name.into(), Box::new(f));
    }

    pub async fn exec(&self, fc: FunctionCall) -> FunctionResult {
        if let Some(h) = self.functions.get(&fc.name) {
            let res = h.call(fc.args).await;
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
