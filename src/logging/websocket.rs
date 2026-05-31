use crate::error::Result;
use crate::logging::events::BotLogEvent;
use crate::logging::LogSink;
use async_trait::async_trait;
use futures_util::SinkExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone)]
pub struct WebSocketSink {
    sender: broadcast::Sender<String>,
}

impl WebSocketSink {
    pub fn new(buffer: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(buffer);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    #[cfg(test)]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

#[async_trait]
impl LogSink for WebSocketSink {
    async fn emit(&self, event: &BotLogEvent) -> Result<()> {
        let payload = serde_json::to_string(event)?;
        let _ = self.sender.send(payload);
        Ok(())
    }
}

pub async fn run_log_websocket_server(host: &str, port: u16, sink: WebSocketSink) -> Result<()> {
    let listener = TcpListener::bind(format!("{host}:{port}")).await?;
    loop {
        let (stream, _addr) = listener.accept().await?;
        let mut receiver = sink.subscribe();

        tokio::spawn(async move {
            let Ok(mut websocket) = accept_async(stream).await else {
                return;
            };

            while let Ok(payload) = receiver.recv().await {
                if websocket.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::events::{BotLogEvent, LogLevel};

    #[tokio::test]
    async fn broadcasts_log_event_to_subscriber() {
        let sink = WebSocketSink::new(16);
        let mut receiver = sink.subscribe();
        sink.emit(&BotLogEvent::new(LogLevel::Info, "test.websocket", "hello"))
            .await
            .unwrap();
        let payload = receiver.recv().await.unwrap();
        assert!(payload.contains("test.websocket"));
    }

    #[tokio::test]
    async fn emit_succeeds_without_subscribers() {
        let sink = WebSocketSink::new(16);
        sink.emit(&BotLogEvent::new(LogLevel::Info, "test.no_subscriber", "hello"))
            .await
            .unwrap();
        assert_eq!(sink.subscriber_count(), 0);
    }
}
