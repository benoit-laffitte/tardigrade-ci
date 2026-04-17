use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use serde_json::json;
use tardigrade_core::{BuildRecord, CompleteBuildRequest, JobStatus, WorkerBuildStatus};
use uuid::Uuid;

use crate::worker_config::WorkerConfig;

/// Abstraction over worker claim/complete HTTP interactions.
#[async_trait]
pub(crate) trait WorkerApi {
    /// Requests next build claim from the GraphQL API.
    async fn claim(&self, graphql_url: &str, worker_id: &str) -> Result<Option<BuildRecord>>;

    /// Reports one build completion payload through the GraphQL API.
    async fn complete(
        &self,
        graphql_url: &str,
        worker_id: &str,
        build_id: Uuid,
        body: &CompleteBuildRequest,
    ) -> Result<()>;
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

    /// Builds HTTP transport from worker config with connection-pool and HTTP/2 tuning.
    pub(crate) fn from_config(config: &WorkerConfig) -> Result<Self> {
        let client = build_worker_http_client(config)?;
        Ok(Self::new(client))
    }
}

/// Builds one shared reqwest client tuned for worker claim/complete loops.
fn build_worker_http_client(config: &WorkerConfig) -> Result<Client> {
    let mut builder = ClientBuilder::new()
        .pool_idle_timeout(std::time::Duration::from_secs(
            config.pool_idle_timeout_secs,
        ))
        .pool_max_idle_per_host(config.pool_max_idle_per_host)
        .timeout(std::time::Duration::from_secs(config.request_timeout_secs))
        .tcp_keepalive(std::time::Duration::from_secs(config.http2_keep_alive_secs))
        .tcp_nodelay(true);

    if config.http2_enabled {
        builder = builder
            .http2_adaptive_window(true)
            .http2_keep_alive_interval(std::time::Duration::from_secs(config.http2_keep_alive_secs))
            .http2_keep_alive_while_idle(true);

        if config.http2_prior_knowledge {
            builder = builder.http2_prior_knowledge();
        }
    } else {
        builder = builder.http1_only();
    }

    builder.build().map_err(Into::into)
}

#[async_trait]
impl WorkerApi for HttpWorkerApi {
    /// Sends worker claim mutation and converts GraphQL payload into a build record.
    async fn claim(&self, graphql_url: &str, worker_id: &str) -> Result<Option<BuildRecord>> {
        let payload: GraphqlEnvelope<ClaimMutationData> = self
            .client
            .post(graphql_url)
            .json(&json!({
                "query": CLAIM_MUTATION,
                "variables": { "workerId": worker_id },
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let data = payload.into_data()?;
        data.worker_claim_build.map(TryInto::try_into).transpose()
    }

    /// Sends worker completion mutation and validates GraphQL success.
    async fn complete(
        &self,
        graphql_url: &str,
        worker_id: &str,
        build_id: Uuid,
        body: &CompleteBuildRequest,
    ) -> Result<()> {
        let status = match body.status {
            WorkerBuildStatus::Success => "SUCCESS",
            WorkerBuildStatus::Failed => "FAILED",
        };

        let payload: GraphqlEnvelope<CompleteMutationData> = self
            .client
            .post(graphql_url)
            .json(&json!({
                "query": COMPLETE_MUTATION,
                "variables": {
                    "workerId": worker_id,
                    "buildId": build_id.to_string(),
                    "status": status,
                    "logLine": body.log_line,
                },
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let data = payload.into_data()?;
        let _ = data.worker_complete_build.id;
        Ok(())
    }
}

/// GraphQL mutation used by workers to claim one build.
const CLAIM_MUTATION: &str = r#"
mutation Claim($workerId: String!) {
  worker_claim_build(workerId: $workerId) {
    id
    job_id
    status
    queued_at
    started_at
    finished_at
    logs
  }
}
"#;

/// GraphQL mutation used by workers to complete one build.
const COMPLETE_MUTATION: &str = r#"
mutation Complete($workerId: String!, $buildId: ID!, $status: GqlWorkerBuildStatus!, $logLine: String) {
  worker_complete_build(workerId: $workerId, buildId: $buildId, status: $status, logLine: $logLine) {
    id
  }
}
"#;

/// GraphQL top-level envelope returned by the API.
#[derive(Debug, Deserialize)]
struct GraphqlEnvelope<T> {
    data: Option<T>,
    errors: Option<Vec<GraphqlErrorEntry>>,
}

impl<T> GraphqlEnvelope<T> {
    /// Extracts data payload or converts GraphQL errors into one anyhow error.
    fn into_data(self) -> Result<T> {
        if let Some(errors) = self.errors
            && !errors.is_empty()
        {
            let messages = errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(anyhow!(messages));
        }

        self.data
            .ok_or_else(|| anyhow!("missing GraphQL data payload"))
    }
}

/// Minimal GraphQL error entry used by worker transport.
#[derive(Debug, Deserialize)]
struct GraphqlErrorEntry {
    message: String,
}

/// GraphQL payload for worker claim mutation.
#[derive(Debug, Deserialize)]
struct ClaimMutationData {
    worker_claim_build: Option<GraphqlBuildRecord>,
}

/// GraphQL payload for worker completion mutation.
#[derive(Debug, Deserialize)]
struct CompleteMutationData {
    worker_complete_build: GraphqlBuildRecordId,
}

/// GraphQL build record shape returned to workers.
#[derive(Debug, Deserialize)]
struct GraphqlBuildRecord {
    id: String,
    job_id: String,
    status: String,
    queued_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    logs: Vec<String>,
}

impl TryFrom<GraphqlBuildRecord> for BuildRecord {
    type Error = anyhow::Error;

    /// Converts GraphQL worker build payload into the core build record model.
    fn try_from(value: GraphqlBuildRecord) -> Result<Self> {
        Ok(Self {
            id: Uuid::parse_str(&value.id)?,
            job_id: Uuid::parse_str(&value.job_id)?,
            status: parse_job_status(&value.status)?,
            queued_at: parse_datetime(&value.queued_at)?,
            started_at: value
                .started_at
                .as_deref()
                .map(parse_datetime)
                .transpose()?,
            finished_at: value
                .finished_at
                .as_deref()
                .map(parse_datetime)
                .transpose()?,
            logs: value.logs,
        })
    }
}

/// GraphQL identifier-only payload returned by completion mutation.
#[derive(Debug, Deserialize)]
struct GraphqlBuildRecordId {
    id: String,
}

/// Parses one RFC3339 timestamp into UTC.
fn parse_datetime(raw: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(raw)?.with_timezone(&Utc))
}

/// Parses GraphQL enum string into the core build lifecycle enum.
fn parse_job_status(raw: &str) -> Result<JobStatus> {
    match raw {
        "PENDING" => Ok(JobStatus::Pending),
        "RUNNING" => Ok(JobStatus::Running),
        "SUCCESS" => Ok(JobStatus::Success),
        "FAILED" => Ok(JobStatus::Failed),
        "CANCELED" => Ok(JobStatus::Canceled),
        _ => Err(anyhow!("unknown build status: {raw}")),
    }
}
