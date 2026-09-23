//! # OpenPet AI Providers
//!
//! Provider abstraction, streaming engine, exponential backoff, and contract testing suite.

pub mod adapters;
pub mod fake;
pub mod provider;
pub mod retry;

pub use adapters::*;
pub use fake::*;
pub use provider::*;
pub use retry::*;

#[cfg(test)]
mod tests {
    use super::*;
    use openpet_types::ChatMessage;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_provider_contract_streaming_success() {
        let fake = FakeChatProvider::new(FakeMode::NormalStream(vec![
            "Hello".into(),
            " world!".into(),
        ]));

        let (tx, mut rx) = mpsc::channel(10);
        let req = ChatRequest {
            messages: vec![ChatMessage::user(Uuid::new_v4(), "hi")],
            system_prompt: None,
            temperature: 0.7,
            max_tokens: Some(100),
        };

        fake.stream_chat(req, tx).await.unwrap();

        let mut collected = String::new();
        while let Some(res) = rx.recv().await {
            let chunk = res.unwrap();
            collected.push_str(&chunk.text);
            if chunk.is_done {
                break;
            }
        }

        assert_eq!(collected, "Hello world!");
    }

    #[tokio::test]
    async fn test_provider_contract_401_authentication_failure() {
        let fake = FakeChatProvider::new(FakeMode::FailWith(
            openpet_types::ProviderError::AuthenticationFailed,
        ));

        let (tx, _rx) = mpsc::channel(10);
        let req = ChatRequest {
            messages: vec![],
            system_prompt: None,
            temperature: 0.7,
            max_tokens: None,
        };

        let result = fake.stream_chat(req, tx).await;
        assert_eq!(
            result,
            Err(openpet_types::ProviderError::AuthenticationFailed)
        );
    }

    #[tokio::test]
    async fn test_provider_contract_retry_backoff() {
        let fake = FakeChatProvider::new(FakeMode::FailThenSucceed {
            failures_before_success: 2,
            success_chunks: vec!["recovered".into()],
        });

        let op = || async {
            let (tx, mut rx) = mpsc::channel(10);
            let req = ChatRequest {
                messages: vec![],
                system_prompt: None,
                temperature: 0.7,
                max_tokens: None,
            };
            fake.stream_chat(req, tx).await?;
            let mut text = String::new();
            while let Some(res) = rx.recv().await {
                let chunk = res?;
                text.push_str(&chunk.text);
            }
            Ok(text)
        };

        let res = retry_with_backoff(op, 3, Duration::from_millis(10)).await;
        assert_eq!(res.unwrap(), "recovered");
        assert_eq!(fake.call_count(), 3);
    }
}
