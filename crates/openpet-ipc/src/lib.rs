//! # OpenPet IPC
//!
//! Secure, length-delimited JSON Named Pipe IPC between `openpet-host` and `openpet-control`.

use async_trait::async_trait;
use openpet_types::{IpcRequest, IpcResponse};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const IPC_PROTOCOL_VERSION: u32 = 1;

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Protocol handshake failed: {0}")]
    HandshakeFailed(String),
    #[error("Connection closed prematurely")]
    ConnectionClosed,
    #[error("Message payload exceeded security limit of {0} bytes")]
    MessageTooLarge(usize),
}

/// Computes the standard named pipe path for the current user session.
pub fn default_pipe_name() -> String {
    let user = std::env::var("USERNAME").unwrap_or_else(|_| "default".to_string());
    // Sanitize user name to prevent path traversal in pipe name
    let clean_user: String = user.chars().filter(|c| c.is_alphanumeric()).collect();
    format!(r"\\.\pipe\OpenPet-{}-control-v1", clean_user)
}

/// Maximum payload size allowed for an IPC message (16 MB) to prevent memory exhaustion attacks.
pub const MAX_IPC_PAYLOAD_SIZE: usize = 16 * 1024 * 1024;

/// Encodes an IPC message into length-prefixed binary JSON frame.
pub fn encode_frame<T: serde::Serialize>(msg: &T) -> Result<Vec<u8>, IpcError> {
    let json_bytes = serde_json::to_vec(msg)?;
    if json_bytes.len() > MAX_IPC_PAYLOAD_SIZE {
        return Err(IpcError::MessageTooLarge(json_bytes.len()));
    }
    let mut frame = Vec::with_capacity(4 + json_bytes.len());
    let len = json_bytes.len() as u32;
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&json_bytes);
    Ok(frame)
}

/// Reads a single length-prefixed frame from any async reader.
pub async fn read_frame<R: AsyncReadExt + Unpin, T: serde::de::DeserializeOwned>(
    reader: &mut R,
) -> Result<T, IpcError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_IPC_PAYLOAD_SIZE {
        return Err(IpcError::MessageTooLarge(len));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    let msg: T = serde_json::from_slice(&payload)?;
    Ok(msg)
}

/// Writes a single length-prefixed frame to any async writer.
pub async fn write_frame<W: AsyncWriteExt + Unpin, T: serde::Serialize>(
    writer: &mut W,
    msg: &T,
) -> Result<(), IpcError> {
    let frame = encode_frame(msg)?;
    writer.write_all(&frame).await?;
    writer.flush().await?;
    Ok(())
}

/// Request dispatcher interface implemented by the OpenPet Host runtime.
#[async_trait]
pub trait IpcRequestHandler: Send + Sync {
    async fn handle_request(&self, request: IpcRequest) -> IpcResponse;
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpet_types::AppSettings;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_frame_codec_roundtrip() {
        let request = IpcRequest::GetSettings;
        let mut buffer = Vec::new();
        write_frame(&mut buffer, &request).await.unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded: IpcRequest = read_frame(&mut cursor).await.unwrap();
        assert_eq!(decoded, IpcRequest::GetSettings);
    }

    #[tokio::test]
    async fn test_response_codec_roundtrip() {
        let response = IpcResponse::Settings(AppSettings::default());
        let mut buffer = Vec::new();
        write_frame(&mut buffer, &response).await.unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded: IpcResponse = read_frame(&mut cursor).await.unwrap();
        assert_eq!(decoded, response);
    }
}
