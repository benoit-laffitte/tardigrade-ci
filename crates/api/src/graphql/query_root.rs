use async_graphql::{Context, Error as GraphQLError, Object};
use axum::http::StatusCode;

use super::{
    GqlBuildRecord, GqlDashboardSnapshot, GqlHealthResponse, GqlJobDefinition, GqlLiveResponse,
    GqlPluginAuthorizationCheckResponse, GqlPluginInfo, GqlPluginPolicyResponse, GqlReadyResponse,
    GqlRuntimeMetrics, GqlScmWebhookRejectionEntry, GqlWorkerInfo, gql_err_from_api,
};
use crate::ApiState;

/// GraphQL query root exposing read-oriented CI operations.
pub(crate) struct QueryRoot;

#[Object(rename_fields = "snake_case")]
impl QueryRoot {
    /// Returns service identity and health status.
    async fn health(&self, ctx: &Context<'_>) -> GqlHealthResponse {
        let state = ctx.data_unchecked::<ApiState>();
        GqlHealthResponse {
            status: "ok".to_string(),
            service: state.service_name.clone(),
        }
    }

    /// Returns process liveness status.
    async fn live(&self) -> GqlLiveResponse {
        GqlLiveResponse {
            status: "alive".to_string(),
        }
    }

    /// Returns readiness status after dependency checks.
    async fn ready(&self, ctx: &Context<'_>) -> Result<GqlReadyResponse, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        state.service.is_ready().await.map_err(gql_err_from_api)?;
        Ok(GqlReadyResponse {
            status: "ready".to_string(),
        })
    }

    /// Returns jobs list sorted by creation time.
    async fn jobs(&self, ctx: &Context<'_>) -> Result<Vec<GqlJobDefinition>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let jobs = state.service.list_jobs().await.map_err(gql_err_from_api)?;
        Ok(jobs.into_iter().map(Into::into).collect())
    }

    /// Returns builds list sorted by queue time.
    async fn builds(&self, ctx: &Context<'_>) -> Result<Vec<GqlBuildRecord>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let builds = state
            .service
            .list_builds()
            .await
            .map_err(gql_err_from_api)?;
        Ok(builds.into_iter().map(Into::into).collect())
    }

    /// Returns worker telemetry and current load.
    async fn workers(&self, ctx: &Context<'_>) -> Result<Vec<GqlWorkerInfo>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let workers = state.service.list_workers().map_err(gql_err_from_api)?;
        Ok(workers.into_iter().map(Into::into).collect())
    }

    /// Returns plugin lifecycle inventory currently loaded in API state.
    async fn plugins(&self, ctx: &Context<'_>) -> Result<Vec<GqlPluginInfo>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let plugins = state.list_plugins().map_err(gql_err_from_status)?;
        Ok(plugins.into_iter().map(Into::into).collect())
    }

    /// Returns granted plugin capabilities for one context with global fallback.
    async fn plugin_policy(
        &self,
        ctx: &Context<'_>,
        context: Option<String>,
    ) -> Result<GqlPluginPolicyResponse, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let policy = state
            .plugin_policy(context.as_deref())
            .map_err(gql_err_from_status)?;
        Ok(policy.into())
    }

    /// Returns authorization decision for one plugin in one context.
    async fn plugin_authorization_check(
        &self,
        ctx: &Context<'_>,
        plugin_name: String,
        context: Option<String>,
    ) -> Result<GqlPluginAuthorizationCheckResponse, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let decision = state
            .plugin_authorization_check(plugin_name.trim(), context.as_deref())
            .map_err(gql_err_from_status)?;
        Ok(decision.into())
    }

    /// Returns runtime reliability counters.
    async fn metrics(&self, ctx: &Context<'_>) -> GqlRuntimeMetrics {
        let state = ctx.data_unchecked::<ApiState>();
        state.service.metrics_snapshot().into()
    }

    /// Returns recent SCM webhook rejection diagnostics.
    async fn scm_webhook_rejections(
        &self,
        ctx: &Context<'_>,
        provider: Option<String>,
        repository_url: Option<String>,
        limit: Option<i32>,
    ) -> Vec<GqlScmWebhookRejectionEntry> {
        let state = ctx.data_unchecked::<ApiState>();
        let limit = limit.unwrap_or(20).max(0) as usize;
        state
            .service
            .list_scm_webhook_rejections(provider.as_deref(), repository_url.as_deref(), limit)
            .into_iter()
            .map(Into::into)
            .collect()
    }

    /// Returns builds currently moved to dead-letter set.
    async fn dead_letter_builds(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<GqlBuildRecord>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let builds = state
            .service
            .list_dead_letter_builds()
            .await
            .map_err(gql_err_from_api)?;
        Ok(builds.into_iter().map(Into::into).collect())
    }

    /// Returns full dashboard snapshot in a single request.
    async fn dashboard_snapshot(
        &self,
        ctx: &Context<'_>,
    ) -> Result<GqlDashboardSnapshot, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let jobs = state.service.list_jobs().await.map_err(gql_err_from_api)?;
        let builds = state
            .service
            .list_builds()
            .await
            .map_err(gql_err_from_api)?;
        let workers = state.service.list_workers().map_err(gql_err_from_api)?;
        let dead_letter_builds = state
            .service
            .list_dead_letter_builds()
            .await
            .map_err(gql_err_from_api)?;

        Ok(GqlDashboardSnapshot {
            jobs: jobs.into_iter().map(Into::into).collect(),
            builds: builds.into_iter().map(Into::into).collect(),
            workers: workers.into_iter().map(Into::into).collect(),
            metrics: state.service.metrics_snapshot().into(),
            dead_letter_builds: dead_letter_builds.into_iter().map(Into::into).collect(),
        })
    }
}

/// Converts an HTTP status-style failure into a GraphQL transport error.
fn gql_err_from_status(status: StatusCode) -> GraphQLError {
    GraphQLError::new(format!("request failed with status {}", status.as_u16()))
}
