use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use crate::error::{DocsiteError, Result};

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub final_url: String,
    pub content_type: Option<String>,
    pub body: String,
}

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
    retry_attempts: usize,
    rate_limit: Duration,
    last_request: Arc<Mutex<Option<Instant>>>,
}

impl HttpClient {
    pub fn new(retry_attempts: usize, rate_limit_ms: u64) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("docsite-to-md/0.1.0 (+https://github.com/K4Lok/docsite-to-md)")
            .build()
            .map_err(|error| DocsiteError::Request {
                url: "<client-builder>".to_string(),
                message: error.to_string(),
            })?;

        Ok(Self {
            client,
            retry_attempts,
            rate_limit: Duration::from_millis(rate_limit_ms),
            last_request: Arc::new(Mutex::new(None)),
        })
    }

    async fn throttle(&self) {
        if self.rate_limit.is_zero() {
            return;
        }

        let mut guard = self.last_request.lock().await;
        if let Some(last_request) = *guard {
            let elapsed = last_request.elapsed();
            if elapsed < self.rate_limit {
                tokio::time::sleep(self.rate_limit - elapsed).await;
            }
        }
        *guard = Some(Instant::now());
    }

    pub async fn fetch_text(&self, url: &str) -> Result<FetchResult> {
        let mut last_error = None;

        for attempt in 0..=self.retry_attempts {
            self.throttle().await;

            match self.client.get(url).send().await {
                Ok(response) => {
                    let status = response.status();
                    let final_url = response.url().to_string();
                    let content_type = response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .map(ToString::to_string);

                    if !status.is_success() {
                        if attempt == self.retry_attempts {
                            return Err(DocsiteError::HttpStatus {
                                url: url.to_string(),
                                status: status.as_u16(),
                            });
                        }
                    } else {
                        let body =
                            response
                                .text()
                                .await
                                .map_err(|error| DocsiteError::Request {
                                    url: url.to_string(),
                                    message: error.to_string(),
                                })?;

                        return Ok(FetchResult {
                            final_url,
                            content_type,
                            body,
                        });
                    }
                }
                Err(error) => {
                    last_error = Some(error.to_string());
                }
            }

            if attempt < self.retry_attempts {
                let delay = 50_u64.saturating_mul(2_u64.pow(attempt as u32));
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
        }

        Err(DocsiteError::Request {
            url: url.to_string(),
            message: last_error.unwrap_or_else(|| "unknown request failure".to_string()),
        })
    }
}
