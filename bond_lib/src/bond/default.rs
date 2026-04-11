// use crate::agent::{agent::Agent, function::Functions};

// use crate::bond::functions;

// pub async fn default() -> (Agent,) {
// let mut f = Functions::new();
//     let mut m = functions::manual::Manual::new();

//     functions::shell::register(&mut f, &mut m);
//     functions::text::register(&mut f, &mut m);
//     functions::tmux::register(&mut f, &mut m);
//     functions::web::register(&mut f, &mut m);
//     functions::knowledge_base::register(&mut f, &mut m);
//     f.register("manual", m);

//     let agent = Agent::new(f).await;

//     return (agent,)
// }

use crate::{
    agent::{
        agent::{Agent, AgentAction},
        function::Functions,
        llm::{openai::OAILLM, openrouter::OpenRouterLLM},
    },
    bond::shell::Shell,
};
use std::{env, fs, path::PathBuf};

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

    let f = Functions::new();
    let llm = OpenRouterLLM::new(&model, &provider.api_key, &providers).await;
    let agent = Agent::new(llm, f).await;

    let agent_ctl = agent.new_ctl();

    for prompt in shell.load_prompts() {
        // println!("{:?} {:?}", prompt.name(), prompt.read());
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
        .send(AgentAction::SendNewChannel {
            channel: "main".to_string(),
            template: "template_main".to_string(),
        })
        .unwrap();

    return (agent,);
}
