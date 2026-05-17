use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    bond::{config::Config, functions, prompts},
};

// ===========================================
// FUNCTION
// ===========================================

/*
Function API:
16 ARGS
- 0: Reserved | Source
- 1: Reserved | ShellPath
- 2: Name
- 3: Method
- 4..=16: Available
*/

pub trait Function: Send + Sync {
    fn name(&self) -> String;
    fn source(&self) -> &str;
    fn call(&self, shell: &Shell, args: &[String]) -> String;

    fn info(&self, shell: &Shell) -> String {
        let mut args = vec![String::new(); 16];
        args[3] = "INFO".to_string();
        self.call(shell, &args)
    }
}

struct BuiltinFunction {
    name: String,
    f: fn(&[String]) -> String,
}

impl Function for BuiltinFunction {
    fn name(&self) -> String {
        return self.name.to_uppercase();
    }

    fn source(&self) -> &str {
        return "builtin";
    }

    fn call(&self, shell: &Shell, args: &[String]) -> String {
        assert_eq!(args.len(), 16, "BuiltinFunction::call requires exactly 16 arguments");

        let mut args_ext = args.to_vec();
        args_ext[0] = "builtin".to_string();
        args_ext[1] = shell.get_path().join(":");
        args_ext[2] = self.name.to_uppercase();
        args_ext[3] = args_ext[3].to_uppercase();

        (self.f)(&args_ext)
    }
}

struct FileFunction {
    path: String,
    name: String,
}

impl Function for FileFunction {
    fn name(&self) -> String {
        return self.name.to_uppercase();
    }

    fn source(&self) -> &str {
        return &self.path;
    }

    fn call(&self, shell: &Shell, args: &[String]) -> String {
        assert_eq!(args.len(), 16, "FileFunction::call requires exactly 16 arguments");

        let mut args_ext = args.to_vec();
        args_ext[0] = self.path.clone();
        args_ext[1] = shell.get_path().join(":");
        args_ext[2] = self.name.clone();
        args_ext[3] = args_ext[3].to_uppercase();

        let child = Command::new(&self.path)
            .args(args_ext)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn");

        let output = child.wait_with_output().expect("failed to wait");

        let output_text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        output_text
    }
}

// ===========================================
// PROMPT
// ===========================================

pub trait Prompt {
    fn name(&self) -> &str;
    fn source(&self) -> &str;
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

    fn source(&self) -> &str {
        return "builtin";
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

    fn source(&self) -> &str {
        return &self.path;
    }

    fn read(&self) -> String {
        self.value.clone()
    }
}

// ===========================================
// SHELL
// ===========================================

#[derive(Debug, Clone)]
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

    pub fn load_prompts(&self) -> Vec<Box<dyn Prompt>> {
        let mut prompts: Vec<Box<dyn Prompt>> = Vec::new();

        prompts.push(Box::new(BuiltinPrompt {
            name: "system".to_string(),
            value: prompts::DEFAULT_SYSTEM.to_string(),
        }));
        prompts.push(Box::new(BuiltinPrompt {
            name: "function_call".to_string(),
            value: prompts::DEFAULT_FUNCTION_CALL.to_string(),
        }));
        prompts.push(Box::new(BuiltinPrompt {
            name: "custom_functions".to_string(),
            value: prompts::CUSTOM_FUNCTIONS.to_string(),
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

    pub fn load_functions(&self) -> Vec<Box<dyn Function>> {
        let mut functions: Vec<Box<dyn Function>> = Vec::new();

        functions.push(Box::new(BuiltinFunction {
            name: "shell".to_string(),
            f: functions::shell::call,
        }));

        for file_path in self.list_files() {
            let name = file_path.file_name().unwrap().to_string_lossy();

            if name.starts_with("function_") {
                functions.push(Box::new(FileFunction {
                    path: file_path.to_string_lossy().to_string(),
                    name: file_path
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .strip_prefix("function_")
                        .unwrap()
                        .to_string(),
                }));
            }
        }

        functions
    }

    // pub fn save_file(&mut self, file_name: &str, value: &str) {

    // }

    // pub fn load_functions(&mut self, ) {

    // }

    // pub fn exec_function(&mut self, ) {

    // }
}
