use axum::{
    Router,
    extract::{
        State,
        ws::{Message as AxumWsMessage, WebSocket, WebSocketUpgrade},
    },
    response::{Html, IntoResponse},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use bond_lib::agent::message::{Message, MessageAuthor, MessageValue};
use bond_lib::agent::{
    agent::{AgentAction, AgentCtl},
    message::MessageExtra,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    SendMessage { channel: String, text: String },
    Cancel { channel: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum WsMessageAuthor {
    Llm,
    Function,
    User,
    System,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum WsMessageType {
    Text,
    FunctionCall,
    FunctionResult,
    Error,
}

#[derive(Debug, Serialize)]
struct WsMessage {
    id: u64,
    author: WsMessageAuthor,
    msg_type: WsMessageType,
    content: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Message { channel: String, message: WsMessage },
    SendTextMessage { channel: String, text: String },
    SendCancel { channel: String },
}

fn serialize_message(msg: Message) -> WsMessage {
    let author = match msg.author {
        MessageAuthor::LLM => WsMessageAuthor::Llm,
        MessageAuthor::Function => WsMessageAuthor::Function,
        MessageAuthor::User => WsMessageAuthor::User,
        MessageAuthor::System => WsMessageAuthor::System,
    };

    let msg_type = match &msg.value {
        MessageValue::Text(_) => WsMessageType::Text,
        MessageValue::TextLLM(_, _, _) => WsMessageType::Text,
        MessageValue::FunctionCall(_) => WsMessageType::FunctionCall,
        MessageValue::FunctionResult(_) => WsMessageType::FunctionResult,
        MessageValue::Error(_) => WsMessageType::Error,
    };

    WsMessage {
        id: msg.id,
        author,
        msg_type,
        content: msg.value.as_str(),
    }
}

pub async fn run(ctl: AgentCtl) {
    let shared_ctl = Arc::new(ctl);

    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("index.html")) }))
        .route("/ws", get(ws_handler))
        .with_state(shared_ctl);

    let addr = "127.0.0.1:6969";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");
    println!("Server running at http://{}", addr);

    axum::serve(listener, app).await.expect("Server failed");
}

async fn ws_handler(ws: WebSocketUpgrade, State(ctl): State<Arc<AgentCtl>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(socket, ctl))
}

async fn handle_connection(socket: WebSocket, ctl: Arc<AgentCtl>) {
    println!("Incoming WebSocket connection ready");
    let (mut ws_tx, mut ws_rx) = socket.split();

    let mut tx_ctl = ctl.new_ctl();
    let mut rx_ctl = ctl.new_ctl();

    // Agent → WebSocket
    let send_task = tokio::spawn(async move {
        loop {
            let action = rx_ctl.recv().await;
            let server_msg: Option<ServerMessage> = match action {
                AgentAction::Message { channel, msg } => Some(ServerMessage::Message {
                    channel,
                    message: serialize_message(msg),
                }),
                AgentAction::Error { channel, msg } => Some(ServerMessage::Message {
                    channel,
                    message: serialize_message(Message {
                        id: 0,
                        time: 0,
                        author: MessageAuthor::System,
                        value: MessageValue::Text(format!("ERROR: {}", msg)),
                        extra: MessageExtra::None,
                    }),
                }),

                _ => None,
            };

            if server_msg.is_none() {
                continue;
            }

            let json = serde_json::to_string(&server_msg).unwrap();
            if ws_tx.send(AxumWsMessage::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // WebSocket → Agent
    while let Some(Ok(raw)) = ws_rx.next().await {
        match raw {
            AxumWsMessage::Binary(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                println!("binary frame: {}", text);
            }
            AxumWsMessage::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::SendMessage { channel, text }) => {
                    tx_ctl.send_message(&channel, &text).await;
                }
                Ok(ClientMessage::Cancel { channel }) => {
                    tx_ctl.send_cancel(&channel).await;
                }
                Err(e) => eprintln!("Bad client message: {}", e),
            },
            AxumWsMessage::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    println!("Connection closed");
}
