use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    agent::langs::cmdlang,
    bond::{config::Config, functions, prompts},
};

// ===========================================
// FUNCTION
// ===========================================

pub trait Function: Send + Sync {
    fn name(&self) -> String;
    fn source(&self) -> &str;
    // [Arg0: Source] | [Arg1: ShellPath] | [Arg2: Name] | Arg3/args[0]: Method | ArgN/args[...]: Rest
    // args[0] = method (should be uppercased), args[1..] = rest
    fn call(&self, shell: &Shell, args: &[String]) -> String;

    fn info(&self, shell: &Shell) -> String {
        self.call(shell, &["INFO".to_string()])
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
        let source = "builtin".to_string();
        let paths = shell.get_path().join(":");
        let name = self.name.to_uppercase();

        // args[0] is method, should be uppercased
        let method = match args.first() {
            Some(m) => m.to_uppercase(),
            None => return "BuiltinFunction: missing method".to_string(),
        };
        let rest = &args[1..];

        let mut args_ext = vec![source, paths, name, method];
        args_ext.extend_from_slice(rest);

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
        let source = self.path.clone();
        let paths = shell.get_path().join(":");
        let name = self.name.clone();

        let method = match args.first() {
            Some(m) => m.to_uppercase(),
            None => return "FileFunction: missing method".to_string(),
        };
        let rest = &args[1..];

        let mut args_ext = vec![source, paths, name, method];
        args_ext.extend_from_slice(rest);

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
            name: "cmdlang".to_string(),
            value: cmdlang::CMDLANG_PROMPT.to_string(),
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
