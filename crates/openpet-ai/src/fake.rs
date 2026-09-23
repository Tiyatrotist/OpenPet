use crate::provider::{ChatChunk, ChatProvider, ChatRequest};
use async_trait::async_trait;
use openpet_types::{ProviderCapabilities, ProviderError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Simulated provider behavior modes for contract tests.
#[derive(Debug, Clone)]
pub enum FakeMode {
    NormalStream(Vec<String>),
    FailWith(ProviderError),
    FailThenSucceed {
        failures_before_success: usize,
        success_chunks: Vec<String>,
    },
    SlowStream {
        chunks: Vec<String>,
        delay_ms: u64,
    },
}

/// Offline, deterministic fake provider for integration tests and contract verification.
pub struct FakeChatProvider {
    mode: FakeMode,
    call_count: Arc<AtomicUsize>,
}

impl FakeChatProvider {
    pub fn new(mode: FakeMode) -> Self {
        Self {
            mode,
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ChatProvider for FakeChatProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            tool_calling: true,
            structured_output: true,
            vision_input: false,
            image_generation: false,
            model_discovery: false,
        }
    }

    fn name(&self) -> &'static str {
        "fake"
    }

    async fn stream_chat(
        &self,
        _request: ChatRequest,
        sender: mpsc::Sender<Result<ChatChunk, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let calls = self.call_count.fetch_add(1, Ordering::SeqCst);

        match &self.mode {
            FakeMode::NormalStream(chunks) => {
                for chunk in chunks {
                    let _ = sender
                        .send(Ok(ChatChunk {
                            text: chunk.clone(),
                            is_done: false,
                        }))
                        .await;
                }
                let _ = sender
                    .send(Ok(ChatChunk {
                        text: String::new(),
                        is_done: true,
                    }))
                    .await;
                Ok(())
            }
            FakeMode::FailWith(err) => Err(err.clone()),
            FakeMode::FailThenSucceed {
                failures_before_success,
                success_chunks,
            } => {
                if calls < *failures_before_success {
                    Err(ProviderError::RateLimited {
                        retry_after_secs: Some(1),
                    })
                } else {
                    for chunk in success_chunks {
                        let _ = sender
                            .send(Ok(ChatChunk {
                                text: chunk.clone(),
                                is_done: false,
                            }))
                            .await;
                    }
                    let _ = sender
                        .send(Ok(ChatChunk {
                            text: String::new(),
                            is_done: true,
                        }))
                        .await;
                    Ok(())
                }
            }
            FakeMode::SlowStream { chunks, delay_ms } => {
                for chunk in chunks {
                    tokio::time::sleep(tokio::time::Duration::from_millis(*delay_ms)).await;
                    let _ = sender
                        .send(Ok(ChatChunk {
                            text: chunk.clone(),
                            is_done: false,
                        }))
                        .await;
                }
                let _ = sender
                    .send(Ok(ChatChunk {
                        text: String::new(),
                        is_done: true,
                    }))
                    .await;
                Ok(())
            }
        }
    }
}
