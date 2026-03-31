use async_graphql::{Context, Error as GraphQLError, ID, Object};

use super::{
	GqlBuildRecord, GqlCreateJobInput, GqlJobDefinition, GqlWorkerBuildStatus, gql_err_from_api,
	parse_id_as_uuid,
};
use crate::{ApiState, CreateJobRequest, WorkerBuildStatus};

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
		let state = ctx.data_unchecked::<ApiState>();
		let job = state
			.service
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
		let state = ctx.data_unchecked::<ApiState>();
		let job_uuid = parse_id_as_uuid(&job_id)?;
		let build = state
			.service
			.run_job(job_uuid)
			.await
			.map_err(gql_err_from_api)?;

		if state.run_embedded_worker {
			let service = state.service.clone();
			tokio::spawn(async move {
				let _ = service.process_next_build().await;
			});
		}

		Ok(build.into())
	}

	/// Cancels one build by id.
	async fn cancel_build(
		&self,
		ctx: &Context<'_>,
		build_id: ID,
	) -> Result<GqlBuildRecord, GraphQLError> {
		let state = ctx.data_unchecked::<ApiState>();
		let build_uuid = parse_id_as_uuid(&build_id)?;
		let build = state
			.service
			.cancel_build(build_uuid)
			.await
			.map_err(gql_err_from_api)?;
		Ok(build.into())
	}

	/// Claims one build for worker and marks it running.
	async fn worker_claim_build(
		&self,
		ctx: &Context<'_>,
		worker_id: String,
	) -> Result<Option<GqlBuildRecord>, GraphQLError> {
		let state = ctx.data_unchecked::<ApiState>();
		let build = state
			.service
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
		let state = ctx.data_unchecked::<ApiState>();
		let build_uuid = parse_id_as_uuid(&build_id)?;
		let status = match status {
			GqlWorkerBuildStatus::Success => WorkerBuildStatus::Success,
			GqlWorkerBuildStatus::Failed => WorkerBuildStatus::Failed,
		};

		let build = state
			.service
			.complete_build_for_worker(&worker_id, build_uuid, status, log_line)
			.await
			.map_err(gql_err_from_api)?;
		Ok(build.into())
	}
}
