use openpet_types::ProviderError;
use rand::Rng;
use std::future::Future;
use std::time::Duration;
use tracing::warn;

/// Execute an asynchronous operation with exponential backoff and random jitter.
pub async fn retry_with_backoff<F, Fut, T>(
    mut op: F,
    max_retries: u32,
    initial_delay: Duration,
) -> Result<T, ProviderError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ProviderError>>,
{
    let mut attempt = 0;
    let mut current_delay = initial_delay;

    loop {
        match op().await {
            Ok(val) => return Ok(val),
            Err(err) => {
                attempt += 1;
                // Check if error is transient / retryable
                let is_retryable = matches!(
                    err,
                    ProviderError::RateLimited { .. }
                        | ProviderError::Network(_)
                        | ProviderError::Timeout(_)
                        | ProviderError::ProviderUnavailable
                );

                if !is_retryable || attempt > max_retries {
                    return Err(err);
                }

                let jitter: f32 = rand::thread_rng().gen_range(0.8..1.2);
                let sleep_duration = current_delay.mul_f32(jitter);

                warn!(
                    "Provider call failed (attempt {}/{}). Retrying after {:?}. Error: {}",
                    attempt, max_retries, sleep_duration, err
                );

                tokio::time::sleep(sleep_duration).await;
                current_delay = current_delay.saturating_mul(2);
            }
        }
    }
}
