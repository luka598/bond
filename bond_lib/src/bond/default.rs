use crate::{
    agent::{
        agent::{Agent, AgentAction},
        function::Functions,
        llm::openrouter::OpenRouterLLM,
    },
    bond::shell::Shell,
};
use std::{env, fs, path::PathBuf, sync::Arc};

pub fn resolve_home() -> String {
    let path: PathBuf = {
        #[cfg(target_os = "linux")]
        {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap()).join(".config"))
        }

        #[cfg(target_os = "windows")]
        {
            env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env::var_os("USERPROFILE").unwrap())
                        .join("AppData")
                        .join("Roaming")
                })
        }

        #[cfg(target_os = "macos")]
        {
            PathBuf::from(env::var_os("HOME").unwrap())
                .join("Library")
                .join("Application Support")
        }
    };

    path.to_string_lossy().into_owned()
}

pub fn gen_meta_prompt(shell: &Shell) -> String {
    let prompts: String = shell
        .load_prompts()
        .iter()
        .map(|x| format!("{}: {}", x.name(), x.source()))
        .fold(String::new(), |a, b| format!("{}\n{}", a, b));

    let functions: String = shell
        .load_functions()
        .iter()
        .map(|x| format!("{}: {} | {}", x.name(), x.source(), x.info(shell)))
        .fold(String::new(), |a, b| format!("{}\n{}", a, b));

    format!(
        "Bond paths:\n\nPrompts loaded:\n{}\nNamespaces (functions):\n{}",
        prompts, functions
    )
}

pub async fn default(home_path: Option<&str>) -> (Agent,) {
    let mut shell = Shell::new();

    let global_home = PathBuf::from(match home_path {
        Some(x) => x.to_string(),
        None => resolve_home(),
    })
    .join("bond");

    let local_home = PathBuf::from(".bond");

    fs::create_dir_all(&global_home).unwrap();
    fs::create_dir_all(&local_home).unwrap();

    shell.add_path(&global_home.to_string_lossy().into_owned());
    shell.add_path(&local_home.to_string_lossy().into_owned());

    let conf = shell.load_config();
    let model = &conf.model;
    let provider = conf
        .providers
        .iter()
        .find(|x| x.name == conf.provider)
        .unwrap();
    let providers: Vec<String> = provider
        .extra
        .get("providers")
        .unwrap_or(&"".to_string())
        .replace(" ", "")
        .split(",")
        .map(|x| x.to_string())
        .filter(|x| x != "")
        .collect();

    let mut f = Functions::new();
    for function in shell.load_functions() {
        let shell_ref = shell.clone();
        let name = function.name();
        f.register(
            name,
            Arc::new(move |_name: &String, args: &[String]| function.call(&shell_ref, args)),
        );
    }

    let llm = OpenRouterLLM::new(&model, &provider.api_key, &providers).await;
    let agent = Agent::new(llm, f).await;

    let agent_ctl = agent.new_ctl();

    for prompt in shell.load_prompts() {
        agent_ctl
            .tx
            .send(AgentAction::SendSystemPrompt {
                channel: "template_main".to_string(),
                name: prompt.name().to_string(),
                text: prompt.read(),
            })
            .unwrap();
    }

    agent_ctl
        .tx
        .send(AgentAction::SendSystemPrompt {
            channel: "template_main".to_string(),
            name: "meta".to_string(),
            text: gen_meta_prompt(&shell),
        })
        .unwrap();

    agent_ctl
        .tx
        .send(AgentAction::SendNewChannel {
            channel: "main".to_string(),
            template: "template_main".to_string(),
        })
        .unwrap();

    return (agent,);
}
