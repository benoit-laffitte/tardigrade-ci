use async_graphql::{Context, Error as GraphQLError, ErrorExtensions, ID, Object};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use tardigrade_application::ScmWebhookRequest;
use tardigrade_plugins::PluginLifecycleError;

use super::{
    GqlBuildRecord, GqlCreateJobInput, GqlJobDefinition, GqlPluginInfo, GqlPluginPolicyResponse,
    GqlScmPollingTickResponse, GqlUpsertScmPollingConfigInput, GqlUpsertWebhookSecurityConfigInput,
    GqlWebhookHeaderInput, GqlWorkerBuildStatus, gql_err_from_api, parse_id_as_uuid,
};
use crate::{
    ApiAuthContext, ApiAuthStatus, ApiState, CreateJobRequest, UpsertScmPollingConfigRequest,
    UpsertWebhookSecurityConfigRequest, WorkerBuildStatus,
};

/// GraphQL mutation root exposing write-oriented CI operations.
pub(crate) struct MutationRoot;

#[Object(rename_fields = "snake_case")]
impl MutationRoot {
    /// Creates one job definition and persists it.
    async fn create_job(
        &self,
        ctx: &Context<'_>,
        input: GqlCreateJobInput,
    ) -> Result<GqlJobDefinition, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let job = state
            .use_cases
            .create_job(CreateJobRequest {
                name: input.name,
                repository_url: input.repository_url,
                pipeline_path: input.pipeline_path,
                pipeline_yaml: input.pipeline_yaml,
            })
            .await
            .map_err(gql_err_from_api)?;
        Ok(job.into())
    }

    /// Enqueues one build for the specified job id.
    async fn run_job(&self, ctx: &Context<'_>, job_id: ID) -> Result<GqlBuildRecord, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let job_uuid = parse_id_as_uuid(&job_id)?;
        let build = state
            .use_cases
            .run_job(job_uuid)
            .await
            .map_err(gql_err_from_api)?;

        Ok(build.into())
    }

    /// Cancels one build by id.
    async fn cancel_build(
        &self,
        ctx: &Context<'_>,
        build_id: ID,
    ) -> Result<GqlBuildRecord, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let build_uuid = parse_id_as_uuid(&build_id)?;
        let build = state
            .use_cases
            .cancel_build(build_uuid)
            .await
            .map_err(gql_err_from_api)?;
        Ok(build.into())
    }

    /// Loads one built-in plugin into the in-memory lifecycle registry.
    async fn load_plugin(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<GqlPluginInfo, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let plugin = state
            .load_plugin(name.trim())
            .map_err(gql_err_from_plugin)?;
        Ok(plugin.into())
    }

    /// Initializes one previously loaded plugin.
    async fn init_plugin(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<GqlPluginInfo, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let plugin = state
            .init_plugin(name.trim())
            .map_err(gql_err_from_plugin)?;
        Ok(plugin.into())
    }

    /// Executes one initialized plugin for diagnostics.
    async fn execute_plugin(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<GqlPluginInfo, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let plugin = state
            .execute_plugin(name.trim())
            .map_err(gql_err_from_plugin)?;
        Ok(plugin.into())
    }

    /// Unloads one plugin from the lifecycle registry.
    async fn unload_plugin(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<GqlPluginInfo, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let plugin = state
            .unload_plugin(name.trim())
            .map_err(gql_err_from_plugin)?;
        Ok(plugin.into())
    }

    /// Upserts granted plugin capabilities for one policy context.
    async fn upsert_plugin_policy(
        &self,
        ctx: &Context<'_>,
        context: Option<String>,
        granted_capabilities: Vec<String>,
    ) -> Result<GqlPluginPolicyResponse, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let policy = state
            .upsert_plugin_policy(context.as_deref(), granted_capabilities)
            .map_err(gql_err_from_status)?;
        Ok(policy.into())
    }

    /// Upserts webhook verification settings for one repository/provider pair.
    async fn upsert_webhook_security_config(
        &self,
        ctx: &Context<'_>,
        input: GqlUpsertWebhookSecurityConfigInput,
    ) -> Result<bool, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        state
            .upsert_webhook_security_config(UpsertWebhookSecurityConfigRequest {
                repository_url: input.repository_url,
                provider: input.provider.into(),
                secret: input.secret,
                allowed_ips: input.allowed_ips,
            })
            .await
            .map_err(gql_err_from_status)?;
        Ok(true)
    }

    /// Upserts SCM polling settings for one repository/provider pair.
    async fn upsert_scm_polling_config(
        &self,
        ctx: &Context<'_>,
        input: GqlUpsertScmPollingConfigInput,
    ) -> Result<bool, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let interval_secs = u64::try_from(input.interval_secs)
            .map_err(|_| GraphQLError::new("interval_secs must be a non-negative integer"))?;

        state
            .upsert_scm_polling_config(UpsertScmPollingConfigRequest {
                repository_url: input.repository_url,
                provider: input.provider.into(),
                enabled: input.enabled,
                interval_secs,
                branches: input.branches,
            })
            .await
            .map_err(gql_err_from_status)?;
        Ok(true)
    }

    /// Runs one SCM polling tick immediately and returns the enqueue summary.
    async fn run_scm_polling_tick(
        &self,
        ctx: &Context<'_>,
    ) -> Result<GqlScmPollingTickResponse, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let result = state
            .use_cases
            .run_scm_polling_tick()
            .await
            .map_err(gql_err_from_api)?;
        Ok(result.into())
    }

    /// Ingests one SCM webhook through GraphQL by reconstructing the header map.
    async fn ingest_scm_webhook(
        &self,
        ctx: &Context<'_>,
        headers: Vec<GqlWebhookHeaderInput>,
        body: String,
    ) -> Result<bool, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let request = webhook_request_from_graphql_inputs(&headers, body.as_bytes())?;
        match state.use_cases.ingest_scm_webhook_observed(&request).await {
            Ok(()) => Ok(true),
            Err(failure) => {
                if let Some(message) = failure.public_message {
                    return Err(GraphQLError::new(message)
                        .extend_with(|_, ext| ext.set("code", failure.reason_code)));
                }

                Err(gql_err_from_api(failure.api_error))
            }
        }
    }

    /// Claims one build for worker and marks it running.
    async fn worker_claim_build(
        &self,
        ctx: &Context<'_>,
        worker_id: String,
    ) -> Result<Option<GqlBuildRecord>, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let build = state
            .use_cases
            .claim_build_for_worker(&worker_id)
            .await
            .map_err(gql_err_from_api)?;
        Ok(build.map(Into::into))
    }

    /// Completes one worker-owned build and applies retry/dead-letter policy.
    async fn worker_complete_build(
        &self,
        ctx: &Context<'_>,
        worker_id: String,
        build_id: ID,
        status: GqlWorkerBuildStatus,
        log_line: Option<String>,
    ) -> Result<GqlBuildRecord, GraphQLError> {
        ensure_write_auth(ctx)?;
        let state = ctx.data_unchecked::<ApiState>();
        let build_uuid = parse_id_as_uuid(&build_id)?;
        let status = match status {
            GqlWorkerBuildStatus::Success => WorkerBuildStatus::Success,
            GqlWorkerBuildStatus::Failed => WorkerBuildStatus::Failed,
        };

        let build = state
            .use_cases
            .complete_build_for_worker(&worker_id, build_uuid, status, log_line)
            .await
            .map_err(gql_err_from_api)?;
        Ok(build.into())
    }
}

/// Enforces API key authorization on write-oriented GraphQL operations.
fn ensure_write_auth(ctx: &Context<'_>) -> Result<(), GraphQLError> {
    let auth = ctx
        .data_opt::<ApiAuthContext>()
        .cloned()
        .unwrap_or_default();

    match auth.status {
        ApiAuthStatus::Disabled | ApiAuthStatus::Verified => Ok(()),
        ApiAuthStatus::Missing => Err(GraphQLError::new("api key is required for this operation")
            .extend_with(|_, ext| {
                ext.set("code", "unauthorized");
                ext.set("http_status", 401);
            })),
        ApiAuthStatus::Invalid => Err(GraphQLError::new("provided api key is invalid")
            .extend_with(|_, ext| {
                ext.set("code", "forbidden");
                ext.set("http_status", 403);
            })),
    }
}

/// Maps plugin lifecycle failures to structured GraphQL errors.
fn gql_err_from_plugin(error: PluginLifecycleError) -> GraphQLError {
    match error {
        PluginLifecycleError::DuplicateName => {
            GraphQLError::new("plugin already loaded with same name")
                .extend_with(|_, ext| ext.set("code", "plugin_duplicate"))
        }
        PluginLifecycleError::NotFound | PluginLifecycleError::UnknownPlugin => {
            GraphQLError::new("plugin was not found in registry or catalog")
                .extend_with(|_, ext| ext.set("code", "plugin_not_found"))
        }
        PluginLifecycleError::InvalidState => {
            GraphQLError::new("plugin lifecycle transition is not allowed in current state")
                .extend_with(|_, ext| ext.set("code", "plugin_invalid_state"))
        }
        PluginLifecycleError::UnauthorizedCapability(capability) => {
            GraphQLError::new(format!("plugin capability {:?} is not granted", capability))
                .extend_with(|_, ext| ext.set("code", "plugin_unauthorized_capability"))
        }
        PluginLifecycleError::ExecutionPanicked => {
            GraphQLError::new("plugin execution panicked and was safely contained")
                .extend_with(|_, ext| ext.set("code", "plugin_execution_panicked"))
        }
        PluginLifecycleError::ExecutionFailed => GraphQLError::new("plugin execution failed")
            .extend_with(|_, ext| ext.set("code", "plugin_execution_failed")),
        PluginLifecycleError::ManifestIo => GraphQLError::new("plugin manifest could not be read")
            .extend_with(|_, ext| ext.set("code", "plugin_manifest_io")),
        PluginLifecycleError::ManifestParse => GraphQLError::new("plugin manifest is invalid")
            .extend_with(|_, ext| ext.set("code", "plugin_manifest_parse")),
    }
}

/// Converts an HTTP status-style failure into a GraphQL transport error.
fn gql_err_from_status(status: StatusCode) -> GraphQLError {
    GraphQLError::new(format!("request failed with status {}", status.as_u16()))
}

/// Builds transport-neutral webhook command input from GraphQL header/body values.
fn webhook_request_from_graphql_inputs(
    headers: &[GqlWebhookHeaderInput],
    body: &[u8],
) -> Result<ScmWebhookRequest, GraphQLError> {
    let mut normalized = Vec::with_capacity(headers.len());
    for header in headers {
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .map_err(|_| GraphQLError::new(format!("invalid header name: {}", header.name)))?;
        let value = HeaderValue::from_str(&header.value)
            .map_err(|_| GraphQLError::new(format!("invalid header value for {}", header.name)))?;
        let raw_value = value
            .to_str()
            .map_err(|_| GraphQLError::new(format!("invalid header value for {}", header.name)))?;
        normalized.push((name.as_str().to_string(), raw_value.to_string()));
    }

    Ok(ScmWebhookRequest::from_parts(normalized, body.to_vec()))
}
