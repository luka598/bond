use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

use crate::agent::channels::Channels;
use crate::agent::function::Functions;
use crate::agent::llm::openai::OAILLM;
use crate::agent::llm::openrouter::OpenRouterLLM;
use crate::agent::message::{Message, MessageAuthor, MessageExtra, MessageValue};

// ========================================
//
// ========================================

fn time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ========================================
//
// ========================================

#[derive(Debug, Clone)]
pub enum AgentAction {
    SendTextMessage {
        channel: String,
        text: String,
    },
    SendRun {
        channel: String,
    },
    SendCancel {
        channel: String,
    },
    SendSystemPrompt {
        channel: String,
        name: String,
        text: String,
    },
    SendNewChannel {
        channel: String,
        template: String,
    },
    Message {
        channel: String,
        msg: Message,
    },
    Busy {
        channel: String,
        busy: bool,
        cancel: bool,
    },
    Error {
        channel: String,
        msg: String,
    },
}

// ========================================
//
// ========================================

pub struct Agent {
    tx: broadcast::Sender<AgentAction>,
    rx: broadcast::Receiver<AgentAction>,

    channels: Arc<Mutex<Channels>>,
    functions: Functions,
    llm: OpenRouterLLM,
}

impl Agent {
    pub async fn new(llm: OpenRouterLLM, functions: Functions) -> Self {
        let (tx, rx) = broadcast::channel::<AgentAction>(256);
        let mut channels = Channels::new();

        channels.add_channel("template_main");

        Self {
            tx,
            rx,

            channels: Arc::new(Mutex::new(channels)),
            functions: functions,
            llm: llm,
        }
    }

    pub fn new_ctl(&self) -> AgentCtl {
        AgentCtl {
            tx: self.tx.clone(),
            rx: self.tx.subscribe(),
        }
    }

    pub async fn run(mut self) {
        loop {
            let action = match self.rx.recv().await {
                Ok(a) => a,
                Err(_) => break,
            };

            match action {
                AgentAction::SendTextMessage { channel, text } => {
                    self.handle_send_text_message(channel, text).await;
                }
                AgentAction::SendSystemPrompt {
                    channel,
                    name,
                    text,
                } => {
                    self.handle_send_system_prompt(channel, name, text).await;
                }
                AgentAction::SendRun { channel } => {
                    self.handle_send_run(channel).await;
                }
                AgentAction::SendCancel { channel } => {
                    self.handle_send_cancel(channel).await;
                }
                AgentAction::SendNewChannel { channel, template } => {
                    self.handle_send_new_channel(channel, template).await;
                }
                AgentAction::Message { .. } => {}
                AgentAction::Busy {
                    channel,
                    busy,
                    cancel,
                } => {}
                AgentAction::Error { channel, msg } => {}
            }
        }
    }

    async fn handle_send_text_message(&mut self, channel_name: String, text: String) {
        let channel = self.channels.lock().await.get_channel(&channel_name);

        let msg = Message {
            id: time(),
            time: time(),
            author: MessageAuthor::User,
            value: MessageValue::Text(text),
            extra: MessageExtra::None,
        };

        channel.add_message(msg.clone());

        let _ = self.tx.send(AgentAction::Message {
            channel: channel_name,
            msg: msg,
        });
    }

    async fn handle_send_system_prompt(&mut self, channel: String, name: String, text: String) {
        let channel = self.channels.lock().await.get_channel(&channel);
        channel.add_message(Message {
            id: time(),
            time: time(),
            author: MessageAuthor::System,
            value: MessageValue::Text(text),
            extra: MessageExtra::None,
        });
    }

    async fn handle_send_new_channel(&mut self, channel: String, template: String) {
        self.channels
            .lock()
            .await
            .clone_channel("template_main", &channel);
    }

    async fn handle_send_run(&mut self, channel_name: String) {
        let channel = self.channels.lock().await.get_channel(&channel_name);
        let llm = self.llm.clone();
        let tx = self.tx.clone();
        let functions = Functions::new();
        tokio::spawn(async move {
            if !channel.try_set_busy() {
                return;
            }

            let mut last_messages_len: usize = 0;
            let mut added_messages: usize = 0;

            loop {
                if channel.get_messages_len() == (last_messages_len + added_messages) {
                    break;
                }
                last_messages_len = channel.get_messages_len();

                let responses = llm.send(&channel.get_messages(), &functions).await;

                for msg in responses.iter() {
                    channel.add_message(msg.clone());

                    let _ = tx.send(AgentAction::Message {
                        channel: channel_name.clone(),
                        msg: msg.clone(),
                    });

                    added_messages += 1;

                    if let MessageValue::FunctionCall(fc) = &msg.value {
                        let fr = functions.exec(fc.clone()).await;
                        channel.add_message(Message {
                            id: msg.id,
                            time: time(),
                            author: MessageAuthor::System,
                            value: MessageValue::FunctionResult(fr),
                            extra: MessageExtra::None,
                        });
                    }
                }
            }

            channel.unset_busy();
        });
    }

    async fn handle_send_cancel(&self, channel: String) {}
}

// ========================================
//
// ========================================

pub struct AgentCtl {
    pub tx: broadcast::Sender<AgentAction>,
    pub rx: broadcast::Receiver<AgentAction>,
}

impl AgentCtl {
    pub fn new_ctl(&self) -> AgentCtl {
        AgentCtl {
            tx: self.tx.clone(),
            rx: self.tx.subscribe(),
        }
    }

    pub async fn send_message(&self, chan: &str, text: &str) {
        self.tx
            .send(AgentAction::SendTextMessage {
                channel: chan.to_string(),
                text: text.to_string(),
            })
            .unwrap();

        self.tx
            .send(AgentAction::SendRun {
                channel: chan.to_string(),
            })
            .unwrap();
    }

    pub async fn send_cancel(&self, chan: &str) {
        self.tx
            .send(AgentAction::SendCancel {
                channel: chan.to_string(),
            })
            .unwrap();
    }

    pub async fn recv(&mut self) -> AgentAction {
        self.rx.recv().await.expect("Failed to recv from agent")
    }
}
