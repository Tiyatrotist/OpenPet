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
    let clean_user: String = user
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    let safe_user = if clean_user.is_empty() {
        "default"
    } else {
        &clean_user
    };
    format!(r"\\.\pipe\OpenPet-{}-control-v1", safe_user)
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

#[cfg(windows)]
pub struct IpcServer;

#[cfg(windows)]
impl IpcServer {
    pub async fn run<H: IpcRequestHandler + 'static>(
        pipe_name: String,
        handler: std::sync::Arc<H>,
        shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<(), IpcError> {
        use std::sync::atomic::Ordering;
        use tokio::net::windows::named_pipe::ServerOptions;

        let mut is_first = true;
        while !shutdown.load(Ordering::SeqCst) {
            let server = match ServerOptions::new()
                .first_pipe_instance(is_first)
                .max_instances(16)
                .create(&pipe_name)
            {
                Ok(s) => {
                    is_first = false;
                    s
                }
                Err(e) => {
                    tracing::error!("Failed to create named pipe server instance: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    continue;
                }
            };

            tokio::select! {
                res = server.connect() => {
                    if let Err(e) = res {
                        tracing::warn!("Named pipe connection error: {}", e);
                        continue;
                    }
                    let handler_clone = handler.clone();
                    tokio::spawn(async move {
                        Self::handle_client(server, handler_clone).await;
                    });
                }
                _ = async {
                    while !shutdown.load(Ordering::SeqCst) {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                } => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_client<H: IpcRequestHandler>(
        mut stream: tokio::net::windows::named_pipe::NamedPipeServer,
        handler: std::sync::Arc<H>,
    ) {
        // Handshake verification
        match read_frame::<_, IpcRequest>(&mut stream).await {
            Ok(IpcRequest::Handshake(hello)) => {
                if hello.protocol_version != IPC_PROTOCOL_VERSION {
                    let err = IpcResponse::Error(format!(
                        "Incompatible protocol version. Expected v{}, got v{}",
                        IPC_PROTOCOL_VERSION, hello.protocol_version
                    ));
                    let _ = write_frame(&mut stream, &err).await;
                    return;
                }
                let server_hello = IpcResponse::Handshake(openpet_types::ServerHello {
                    protocol_version: IPC_PROTOCOL_VERSION,
                    host_version: env!("CARGO_PKG_VERSION").to_string(),
                    capabilities: vec![
                        "pets".into(),
                        "settings".into(),
                        "reminders".into(),
                        "memory".into(),
                        "chat".into(),
                        "privacy".into(),
                    ],
                });
                if let Err(e) = write_frame(&mut stream, &server_hello).await {
                    tracing::warn!("Failed sending server hello: {}", e);
                    return;
                }
            }
            Ok(other) => {
                let resp = handler.handle_request(other).await;
                let _ = write_frame(&mut stream, &resp).await;
            }
            Err(e) => {
                tracing::debug!("Client disconnected before handshake: {}", e);
                return;
            }
        }

        loop {
            match read_frame::<_, IpcRequest>(&mut stream).await {
                Ok(req) => {
                    let resp = handler.handle_request(req).await;
                    if let Err(e) = write_frame(&mut stream, &resp).await {
                        tracing::debug!("Error sending response: {}", e);
                        break;
                    }
                }
                Err(IpcError::Io(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
}

#[cfg(windows)]
pub struct IpcClient {
    stream: tokio::net::windows::named_pipe::NamedPipeClient,
}

#[cfg(windows)]
impl IpcClient {
    pub async fn connect(pipe_name: &str) -> Result<Self, IpcError> {
        use tokio::net::windows::named_pipe::ClientOptions;

        let mut stream = ClientOptions::new().open(pipe_name)?;

        // Send client hello
        let hello = IpcRequest::Handshake(openpet_types::ClientHello {
            protocol_version: IPC_PROTOCOL_VERSION,
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            client_kind: "control_center".to_string(),
        });
        write_frame(&mut stream, &hello).await?;

        let resp: IpcResponse = read_frame(&mut stream).await?;
        match resp {
            IpcResponse::Handshake(_) => Ok(Self { stream }),
            IpcResponse::Error(msg) => Err(IpcError::HandshakeFailed(msg)),
            other => Err(IpcError::HandshakeFailed(format!(
                "Unexpected handshake response: {:?}",
                other
            ))),
        }
    }

    pub async fn request(&mut self, request: &IpcRequest) -> Result<IpcResponse, IpcError> {
        write_frame(&mut self.stream, request).await?;
        read_frame(&mut self.stream).await
    }
}

#[cfg(not(windows))]
pub struct IpcServer;

#[cfg(not(windows))]
impl IpcServer {
    pub async fn run<H: IpcRequestHandler + 'static>(
        _pipe_name: String,
        _handler: std::sync::Arc<H>,
        _shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<(), IpcError> {
        Ok(())
    }
}

#[cfg(not(windows))]
pub struct IpcClient;

#[cfg(not(windows))]
impl IpcClient {
    pub async fn connect(_pipe_name: &str) -> Result<Self, IpcError> {
        Err(IpcError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Named pipes only supported on Windows",
        )))
    }

    pub async fn request(&mut self, _request: &IpcRequest) -> Result<IpcResponse, IpcError> {
        Err(IpcError::ConnectionClosed)
    }
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

    #[tokio::test]
    #[cfg(windows)]
    async fn test_named_pipe_server_client_roundtrip() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        struct TestHandler;
        #[async_trait]
        impl IpcRequestHandler for TestHandler {
            async fn handle_request(&self, request: IpcRequest) -> IpcResponse {
                match request {
                    IpcRequest::Ping => IpcResponse::Pong,
                    IpcRequest::GetSettings => IpcResponse::Settings(AppSettings::default()),
                    _ => IpcResponse::Ack,
                }
            }
        }

        let pipe_name = format!(r"\\.\pipe\OpenPet-TestPipe-{}", uuid::Uuid::new_v4());
        let shutdown = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(TestHandler);

        let pipe_server = pipe_name.clone();
        let shutdown_server = shutdown.clone();
        tokio::spawn(async move {
            let _ = IpcServer::run(pipe_server, handler, shutdown_server).await;
        });

        // Give server brief moment to spin up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut client = IpcClient::connect(&pipe_name)
            .await
            .expect("Client should connect");
        let resp = client
            .request(&IpcRequest::Ping)
            .await
            .expect("Request should succeed");
        assert_eq!(resp, IpcResponse::Pong);

        let resp_settings = client
            .request(&IpcRequest::GetSettings)
            .await
            .expect("Request should succeed");
        assert_eq!(resp_settings, IpcResponse::Settings(AppSettings::default()));

        shutdown.store(true, Ordering::SeqCst);
    }
}
