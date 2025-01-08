use crate::protocol::messages::JSONRPCMessage;
use crate::{Error, Result};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};
use tokio::sync::Mutex;

/// A transport implementation that uses standard input and output for communication.
/// This is useful for local development and CLI applications.
pub struct StdioTransport {
    stdin: Mutex<BufReader<Stdin>>,
    stdout: Mutex<Stdout>,
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioTransport {
    pub fn new() -> Self {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        Self {
            stdin: Mutex::new(BufReader::new(stdin)),
            stdout: Mutex::new(stdout),
        }
    }

    /// Creates a transport with custom stdin and stdout
    /// This is useful for testing or when you want to use different input/output sources
    pub fn with_io(stdin: Stdin, stdout: Stdout) -> Self {
        Self {
            stdin: Mutex::new(BufReader::new(stdin)),
            stdout: Mutex::new(stdout),
        }
    }

    /// Read a single line from stdin
    async fn read_line(&self) -> Result<String> {
        let mut line = String::new();
        let mut stdin = self.stdin.lock().await;

        match stdin.read_line(&mut line).await {
            Ok(0) => Err(Error::Transport("EOF reached".into())),
            Ok(_) => Ok(line),
            Err(e) => Err(Error::Transport(e.to_string().into())),
        }
    }

    /// Write a line to stdout
    async fn write_line(&self, line: &str) -> Result<()> {
        let mut stdout = self.stdout.lock().await;
        stdout
            .write_all(line.as_bytes())
            .await
            .map_err(|e| Error::Transport(e.to_string().into()))?;
        stdout
            .write_all(b"\n")
            .await
            .map_err(|e| Error::Transport(e.to_string().into()))?;
        stdout
            .flush()
            .await
            .map_err(|e| Error::Transport(e.to_string().into()))?;
        Ok(())
    }
}

#[async_trait]
impl super::Transport for StdioTransport {
    async fn send(&self, message: JSONRPCMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        self.write_line(&json).await
    }

    async fn receive(&self) -> Result<JSONRPCMessage> {
        let line = self.read_line().await?;
        let message = serde_json::from_str(&line)?;
        Ok(message)
    }

    fn message_stream(&self) -> Option<Pin<Box<dyn Stream<Item = Result<JSONRPCMessage>> + Send>>> {
        let transport = self.clone();

        Some(Box::pin(async_stream::stream! {
            loop {
                match transport.receive().await {
                    Ok(msg) => yield Ok(msg),
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        }))
    }
}

impl Clone for StdioTransport {
    fn clone(&self) -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::messages::{JSONRPCRequest, JSONRPC_VERSION};
    use tokio::io::{BufReader, BufWriter};

    #[tokio::test]
    async fn test_stdio_transport() {
        let (input_read, mut input_write) = tokio::io::duplex(1024);
        let (output_read, mut output_write) = tokio::io::duplex(1024);

        // Create transport with mock IO
        let transport = StdioTransport {
            stdin: Mutex::new(BufReader::new(input_read)),
            stdout: Mutex::new(output_write),
        };

        // Test message to send
        let test_message = JSONRPCMessage::Request(JSONRPCRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: 1.into(),
            method: "test".to_string(),
            params: None,
        });

        // Send the message
        transport.send(test_message.clone()).await.unwrap();

        // Read what was written to stdout
        let mut buf = String::new();
        let mut reader = BufReader::new(output_read);
        reader.read_line(&mut buf).await.unwrap();

        // Verify the message was written correctly
        let written_message: JSONRPCMessage = serde_json::from_str(&buf).unwrap();
        assert_eq!(
            serde_json::to_value(written_message).unwrap(),
            serde_json::to_value(test_message).unwrap()
        );

        // Write a message to stdin
        let response = JSONRPCMessage::Request(JSONRPCRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: 2.into(),
            method: "response".to_string(),
            params: None,
        });

        let json = serde_json::to_string(&response).unwrap() + "\n";
        input_write.write_all(json.as_bytes()).await.unwrap();
        input_write.flush().await.unwrap();

        // Receive the message
        let received = transport.receive().await.unwrap();
        assert_eq!(
            serde_json::to_value(received).unwrap(),
            serde_json::to_value(response).unwrap()
        );
    }
}
