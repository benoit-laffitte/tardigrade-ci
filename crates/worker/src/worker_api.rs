use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use tardigrade_api::{ClaimBuildResponse, CompleteBuildRequest};

/// Abstraction over worker claim/complete HTTP interactions.
#[async_trait]
pub(crate) trait WorkerApi {
    /// Requests next build claim from API.
    async fn claim(&self, claim_url: &str) -> Result<ClaimBuildResponse>;

    /// Reports one build completion payload to API.
    async fn complete(&self, complete_url: &str, body: &CompleteBuildRequest) -> Result<()>;
}

/// Reqwest-backed implementation of worker API transport.
pub(crate) struct HttpWorkerApi {
    /// Shared HTTP client used for all requests.
    client: Client,
}

impl HttpWorkerApi {
    /// Builds HTTP transport from an existing reqwest client.
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl WorkerApi for HttpWorkerApi {
    /// Sends claim request and decodes claim response body.
    async fn claim(&self, claim_url: &str) -> Result<ClaimBuildResponse> {
        let payload = self
            .client
            .post(claim_url)
            .send()
            .await?
            .error_for_status()?
            .json::<ClaimBuildResponse>()
            .await?;
        Ok(payload)
    }

    /// Sends completion request and validates successful status code.
    async fn complete(&self, complete_url: &str, body: &CompleteBuildRequest) -> Result<()> {
        self.client
            .post(complete_url)
            .json(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
