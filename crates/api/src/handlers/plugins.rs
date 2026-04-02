use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tardigrade_plugins::PluginLifecycleError;

use crate::{
    ApiErrorResponse, ApiState, ListPluginsResponse, LoadPluginRequest, PluginActionResponse,
};

/// Lists plugin lifecycle inventory currently registered in API state.
pub(crate) async fn list_plugins(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ListPluginsResponse>), StatusCode> {
    let plugins = state.list_plugins()?;
    Ok((StatusCode::OK, Json(ListPluginsResponse { plugins })))
}

/// Loads one plugin by name from built-in API catalog.
pub(crate) async fn load_plugin(
    State(state): State<ApiState>,
    Json(payload): Json<LoadPluginRequest>,
) -> Result<(StatusCode, Json<PluginActionResponse>), (StatusCode, Json<ApiErrorResponse>)> {
    if payload.name.trim().is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "invalid_plugin_request",
            "plugin name is required",
        ));
    }

    let plugin = state
        .load_plugin(payload.name.trim())
        .map_err(map_plugin_error)?;

    Ok((
        StatusCode::CREATED,
        Json(PluginActionResponse {
            status: "loaded".to_string(),
            plugin,
        }),
    ))
}

/// Initializes one loaded plugin.
pub(crate) async fn init_plugin(
    Path(name): Path<String>,
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<PluginActionResponse>), (StatusCode, Json<ApiErrorResponse>)> {
    let plugin = state.init_plugin(name.trim()).map_err(map_plugin_error)?;
    Ok((
        StatusCode::OK,
        Json(PluginActionResponse {
            status: "initialized".to_string(),
            plugin,
        }),
    ))
}

/// Executes one initialized plugin for diagnostics.
pub(crate) async fn execute_plugin(
    Path(name): Path<String>,
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<PluginActionResponse>), (StatusCode, Json<ApiErrorResponse>)> {
    let plugin = state.execute_plugin(name.trim()).map_err(map_plugin_error)?;
    Ok((
        StatusCode::OK,
        Json(PluginActionResponse {
            status: "executed".to_string(),
            plugin,
        }),
    ))
}

/// Unloads one plugin from registry lifecycle.
pub(crate) async fn unload_plugin(
    Path(name): Path<String>,
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<PluginActionResponse>), (StatusCode, Json<ApiErrorResponse>)> {
    let plugin = state.unload_plugin(name.trim()).map_err(map_plugin_error)?;
    Ok((
        StatusCode::OK,
        Json(PluginActionResponse {
            status: "unloaded".to_string(),
            plugin,
        }),
    ))
}

/// Maps lifecycle errors to HTTP status and actionable API error payload.
fn map_plugin_error(error: PluginLifecycleError) -> (StatusCode, Json<ApiErrorResponse>) {
    match error {
        PluginLifecycleError::DuplicateName => error_response(
            StatusCode::CONFLICT,
            "plugin_duplicate",
            "plugin already loaded with same name",
        ),
        PluginLifecycleError::NotFound | PluginLifecycleError::UnknownPlugin => error_response(
            StatusCode::NOT_FOUND,
            "plugin_not_found",
            "plugin was not found in registry or catalog",
        ),
        PluginLifecycleError::InvalidState => error_response(
            StatusCode::CONFLICT,
            "plugin_invalid_state",
            "plugin lifecycle transition is not allowed in current state",
        ),
        PluginLifecycleError::UnauthorizedCapability(capability) => error_response(
            StatusCode::FORBIDDEN,
            "plugin_unauthorized_capability",
            &format!("plugin capability {:?} is not granted", capability),
        ),
        PluginLifecycleError::ExecutionPanicked => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "plugin_execution_panicked",
            "plugin execution panicked and was safely contained",
        ),
        PluginLifecycleError::ExecutionFailed => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "plugin_execution_failed",
            "plugin execution failed",
        ),
        PluginLifecycleError::ManifestIo => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "plugin_manifest_io",
            "plugin manifest could not be read",
        ),
        PluginLifecycleError::ManifestParse => error_response(
            StatusCode::BAD_REQUEST,
            "plugin_manifest_parse",
            "plugin manifest is invalid",
        ),
    }
}

/// Creates one standard API error tuple for plugin administration endpoints.
fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
) -> (StatusCode, Json<ApiErrorResponse>) {
    (
        status,
        Json(ApiErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }),
    )
}
