use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use crate::bond::{config::Config, prompts};

// ===========================================
// FUNCTION
// ===========================================

pub trait Function {
    fn call(&self, shell: &Shell, args: Vec<String>) {}
}

// ===========================================
// PROMPT
// ===========================================

pub trait Prompt {
    fn name(&self) -> &str;
    fn read(&self) -> String;
}

struct BuiltinPrompt {
    name: String,
    value: String,
}

impl Prompt for BuiltinPrompt {
    fn name(&self) -> &str {
        &self.name
    }

    fn read(&self) -> String {
        self.value.clone()
    }
}

struct FilePrompt {
    path: String,
    name: String,
    value: String,
}

impl Prompt for FilePrompt {
    fn name(&self) -> &str {
        &self.name
    }

    fn read(&self) -> String {
        self.value.clone()
    }
}

// ===========================================
// SHELL
// ===========================================

pub struct Shell {
    path: Vec<String>,
    functions: HashMap<String, String>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            path: vec![],
            functions: HashMap::new(),
        }
    }

    pub fn add_path(&mut self, path: &str) {
        self.path.push(path.to_string());
    }

    pub fn get_path(&self) -> &[String] {
        &self.path
    }

    pub fn list_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let mut seen = HashSet::new();

        for path in self.path.iter().rev() {
            for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();

                if path.is_file() {
                    let name = path.file_name().unwrap().to_owned();

                    if seen.insert(name) {
                        files.push(path);
                    }
                }
            }
        }

        files
    }

    pub fn load_config(&mut self) -> Config {
        for file_path in self.list_files() {
            let name = file_path.file_name().unwrap().to_string_lossy();

            if name == "config.toml" {
                let content = fs::read_to_string(&file_path).unwrap();
                let content =
                    toml::from_str::<Config>(&content).expect("Failed to load the config");

                return content;
            }
        }

        return Config::default();
    }

    pub fn load_prompts(&mut self) -> Vec<Box<dyn Prompt>> {
        let mut prompts: Vec<Box<dyn Prompt>> = Vec::new();

        prompts.push(Box::new(BuiltinPrompt {
            name: "system".to_string(),
            value: prompts::DEFAULT_SYSTEM.to_string(),
        }));
        prompts.push(Box::new(BuiltinPrompt {
            name: "taglang".to_string(),
            value: prompts::DEFAULT_TAGLANG.to_string(),
        }));
        prompts.push(Box::new(BuiltinPrompt {
            name: "function_call".to_string(),
            value: prompts::DEFAULT_FUNCTION_CALL.to_string(),
        }));
        prompts.push(Box::new(BuiltinPrompt {
            name: "soul".to_string(),
            value: prompts::DEFAULT_SOUL.to_string(),
        }));

        for file_path in self.list_files() {
            let name = file_path.file_name().unwrap().to_string_lossy();

            if name.starts_with("prompt_") {
                let content = fs::read_to_string(&file_path).unwrap();
                prompts.push(Box::new(FilePrompt {
                    path: file_path.to_string_lossy().to_string(),
                    name: file_path
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .strip_prefix("prompt_")
                        .unwrap()
                        .to_string(),
                    value: content,
                }));
            }
        }

        prompts
    }

    pub fn load_functions(&mut self) {
        let mut prompts = Vec::new();

        for file_path in self.list_files() {
            let name = file_path.file_name().unwrap().to_string_lossy();

            if name.starts_with("prompt_") {
                let content = fs::read_to_string(&file_path).unwrap();
                prompts.push(content);
            }
        }
    }

    // pub fn save_file(&mut self, file_name: &str, value: &str) {

    // }

    // pub fn load_functions(&mut self, ) {

    // }

    // pub fn exec_function(&mut self, ) {

    // }
}
