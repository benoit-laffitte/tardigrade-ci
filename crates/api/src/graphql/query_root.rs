use async_graphql::{Context, Error as GraphQLError, Object};

use super::{
	GqlBuildRecord, GqlDashboardSnapshot, GqlHealthResponse, GqlJobDefinition, GqlLiveResponse,
	GqlReadyResponse, GqlRuntimeMetrics, GqlWorkerInfo, gql_err_from_api,
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

	/// Returns runtime reliability counters.
	async fn metrics(&self, ctx: &Context<'_>) -> GqlRuntimeMetrics {
		let state = ctx.data_unchecked::<ApiState>();
		state.service.metrics_snapshot().into()
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
