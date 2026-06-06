use serde_json::Value;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::oneshot;

#[derive(Debug, thiserror::Error)]
pub enum QmpError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("QMP protocol error: {0}")]
    Protocol(String),
    #[error("Command failed: {0}")]
    CommandError(String),
    #[error("Internal channel error")]
    ChannelError,
}

/// A single-connection QMP client with background event reader and
/// per-command oneshot response channels.
pub struct QmpClient {
    writer: tokio::io::BufWriter<tokio::net::unix::OwnedWriteHalf>,
    message_id: u64,
    /// Channel receiver for async QMP events (SHUTDOWN, STOP, etc.)
    pub events: tokio::sync::mpsc::UnboundedReceiver<QmpEvent>,
    /// Sender for command response channels (populated by background reader).
    response_tx: tokio::sync::mpsc::UnboundedSender<(u64, oneshot::Sender<Value>)>,
}

/// An async QMP event from QEMU.
#[derive(Debug, Clone)]
pub struct QmpEvent {
    pub event: String,
    pub data: Value,
}

impl QmpClient {
    /// Connect and perform QMP capability negotiation.
    pub async fn connect(socket_path: &Path) -> Result<Self, QmpError> {
        let stream = UnixStream::connect(socket_path).await?;
        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut writer = tokio::io::BufWriter::new(writer);

        // Read QEMU greeting
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let greeting: Value = serde_json::from_str(&line)
            .map_err(|e| QmpError::Protocol(format!("bad greeting: {e}")))?;
        if greeting.get("QMP").is_none() {
            return Err(QmpError::Protocol("no QMP handshake".into()));
        }

        // Send qmp_capabilities
        let cmd = serde_json::json!({"execute": "qmp_capabilities", "id": 1});
        Self::write_json(&mut writer, &cmd).await?;

        // Read capabilities response
        line.clear();
        reader.read_line(&mut line).await?;
        let resp: Value = serde_json::from_str(&line)?;
        if resp.get("return").is_none() {
            return Err(QmpError::Protocol("qmp_capabilities rejected".into()));
        }

        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let (response_tx, mut response_rx) = tokio::sync::mpsc::unbounded_channel::<(u64, oneshot::Sender<Value>)>();

        // Map of pending commands: id -> oneshot sender
        let mut pending: std::collections::HashMap<u64, oneshot::Sender<Value>> =
            std::collections::HashMap::new();

        // Background reader task
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let val: Value = match serde_json::from_str(&line) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        match (val.get("id"), val.get("event")) {
                            // Command response (has "id")
                            (Some(id), _) => {
                                if let Some(id_num) = id.as_u64() {
                                    if let Some(tx) = pending.remove(&id_num) {
                                        let _ = tx.send(val);
                                    }
                                }
                            }
                            // Async event (has "event", no "id")
                            (None, Some(event_name)) => {
                                let _ = event_tx.send(QmpEvent {
                                    event: event_name.as_str().unwrap_or("").to_string(),
                                    data: val.get("data").cloned().unwrap_or(Value::Null),
                                });
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break,
                }

                // Process new pending registrations
                while let Ok((id, tx)) = response_rx.try_recv() {
                    pending.insert(id, tx);
                }
            }
        });

        Ok(Self {
            writer,
            message_id: 2,
            events: event_rx,
            response_tx,
        })
    }

    /// Send a QMP command and return the "return" value.
    pub async fn command(&mut self, execute: &str, args: Value) -> Result<Value, QmpError> {
        let id = self.message_id;
        self.message_id += 1;

        let (tx, rx) = oneshot::channel();

        // Register the response channel before sending (no race because
        // the background reader processes pending before reading).
        self.response_tx
            .send((id, tx))
            .map_err(|_| QmpError::ChannelError)?;

        let msg = serde_json::json!({
            "execute": execute,
            "arguments": args,
            "id": id,
        });
        Self::write_json(&mut self.writer, &msg).await?;

        let resp = rx.await.map_err(|_| QmpError::ChannelError)?;
        if let Some(err) = resp.get("error") {
            let desc = err["desc"].as_str().unwrap_or("unknown");
            return Err(QmpError::CommandError(desc.to_string()));
        }

        Ok(resp["return"].clone())
    }

    /// Take a screenshot (outputs PNG to QEMU's working directory).
    pub async fn screendump(&mut self, output_path: &str) -> Result<(), QmpError> {
        self.command("screendump", serde_json::json!({"filename": output_path}))
            .await?;
        Ok(())
    }

    /// Query VM status.
    pub async fn query_status(&mut self) -> Result<String, QmpError> {
        let ret = self.command("query-status", serde_json::json!({})).await?;
        Ok(ret["status"].as_str().unwrap_or("unknown").to_string())
    }

    /// Gracefully power down the VM.
    pub async fn system_powerdown(&mut self) -> Result<(), QmpError> {
        self.command("system_powerdown", serde_json::json!({})).await?;
        Ok(())
    }

    /// Send a key event.
    pub async fn send_key(&mut self, key: &str, hold_time_ms: u64) -> Result<(), QmpError> {
        self.command(
            "send-key",
            serde_json::json!({"keys": [{"type": "qcode", "data": key}], "hold-time": hold_time_ms}),
        )
        .await?;
        Ok(())
    }

    async fn write_json(writer: &mut tokio::io::BufWriter<tokio::net::unix::OwnedWriteHalf>, value: &Value) -> Result<(), QmpError> {
        let mut json = serde_json::to_string(value)?;
        json.push('\n');
        writer.write_all(json.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Build a QMP command string (helper for tests).
    pub fn build_command(execute: &str, args: &Value, id: u64) -> String {
        let msg = serde_json::json!({
            "execute": execute,
            "arguments": args,
            "id": id,
        });
        let mut json = serde_json::to_string(&msg).unwrap();
        json.push('\n');
        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_formats_correctly() {
        let cmd = QmpClient::build_command("query-status", &serde_json::json!({}), 1);
        let parsed: Value = serde_json::from_str(&cmd.trim()).unwrap();
        assert_eq!(parsed["execute"], "query-status");
        assert_eq!(parsed["id"], 1);
    }

    #[test]
    fn build_command_with_args() {
        let cmd = QmpClient::build_command(
            "system_powerdown",
            &serde_json::json!({"mode": "shutdown"}),
            42,
        );
        let parsed: Value = serde_json::from_str(&cmd.trim()).unwrap();
        assert_eq!(parsed["arguments"]["mode"], "shutdown");
        assert_eq!(parsed["id"], 42);
    }
}
