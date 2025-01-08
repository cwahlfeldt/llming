use crate::protocol::messages::JSONRPCMessage;
use crate::{Error, Result};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use hyperaxe::ws::{Message, WebSocket, WebSocketStream};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;

const CHANNEL_BUFFER_SIZE: usize = 1024;

pub struct WebSocketTransport {
    ws: Arc<RwLock<WebSocket>>,
    tx: UnboundedSender<JSONRPCMessage>,
    rx: Arc<RwLock<UnboundedReceiver<JSONRPCMessage>>>,
}

impl WebSocketTransport {
    pub fn new(ws: WebSocket) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            ws: Arc::new(RwLock::new(ws)),
            tx,
            rx: Arc::new(RwLock::new(rx)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let ws = self.ws.read().await;
        let mut stream = ws.stream();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(message) = serde_json::from_str::<JSONRPCMessage>(&text) {
                            if tx.send(message).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl super::Transport for WebSocketTransport {
    async fn send(&self, message: JSONRPCMessage) -> Result<()> {
        let text = serde_json::to_string(&message)?;
        let mut ws = self.ws.write().await;
        ws.send(Message::Text(text)).await?;
        Ok(())
    }

    async fn receive(&self) -> Result<JSONRPCMessage> {
        let mut rx = self.rx.write().await;
        rx.recv()
            .await
            .ok_or_else(|| Error::Transport("WebSocket connection closed".into()))
    }

    fn message_stream(&self) -> Option<Pin<Box<dyn Stream<Item = Result<JSONRPCMessage>> + Send>>> {
        let rx = self.rx.clone();

        Some(Box::pin(async_stream::stream! {
            let mut rx = rx.write().await;
            while let Some(msg) = rx.recv().await {
                yield Ok(msg);
            }
        }))
    }
}
