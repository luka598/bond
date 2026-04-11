use crate::agent::message::Message;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct Channel {
    messages: Arc<Mutex<Vec<Message>>>,
    busy: AtomicBool,
    cancel: AtomicBool,
}

impl Channel {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            busy: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
        }
    }

    pub fn add_message(&self, msg: Message) {
        self.messages.lock().unwrap().push(msg);
    }

    pub fn get_messages(&self) -> Vec<Message> {
        self.messages.lock().unwrap().clone()
    }

    pub fn get_messages_len(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    pub fn try_set_busy(&self) -> bool {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn unset_busy(&self) {
        self.busy.store(false, Ordering::Release);
    }
}
pub struct Channels {
    channels: HashMap<String, Arc<Channel>>,
}

impl Channels {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn add_channel(&mut self, channel: &str) {
        self.channels
            .entry(channel.to_string())
            .or_insert_with(|| Arc::new(Channel::new()));
    }

    pub fn rem_channel(&mut self, channel: &str) {
        self.channels.remove(channel);
    }

    pub fn rename_channel(&mut self, channel_old: &str, channel_new: &str) {
        if let Some(channel) = self.channels.remove(channel_old) {
            self.channels.insert(channel_new.to_string(), channel);
        }
    }

    pub fn clone_channel(&mut self, channel_src: &str, channel_dest: &str) {
        let messages = {
            let source = self.channels.get(channel_src).unwrap();
            source.get_messages()
        };

        let dest = self
            .channels
            .entry(channel_dest.to_string())
            .or_insert_with(|| Arc::new(Channel::new()));

        for msg in messages {
            dest.add_message(msg);
        }
    }

    pub fn get_channel(&self, channel: &str) -> Arc<Channel> {
        self.channels
            .get(channel)
            .expect("channel does not exist")
            .clone()
    }
}
