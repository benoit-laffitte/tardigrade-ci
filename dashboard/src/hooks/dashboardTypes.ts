import type { AdminRole, Build, CreateJobInput, DashboardNavItem, DashboardPage, Job } from "../widgets/types";

// Represents worker activity and heartbeat information rendered by dashboard features.
export interface Worker {
  id: string;
  active_builds: number;
  status: string;
  last_seen_at: string;
}

// Captures runtime metric counters projected in snapshot responses.
export interface RuntimeMetrics {
  reclaimed_total: number;
  retry_requeued_total: number;
  ownership_conflicts_total: number;
  dead_letter_total: number;
}

// Enumerates SSE severity values carried by live dashboard events.
export type EventSeverity = "ok" | "error" | "warn" | "info";

// Represents one live event emitted by the dashboard realtime stream.
export interface LiveEvent {
  kind?: string;
  message?: string;
  severity?: EventSeverity;
  job_id?: string;
  build_id?: string;
  worker_id?: string;
  at?: string;
}

// Aggregates snapshot data returned by the dashboard GraphQL query.
export interface DashboardSnapshot {
  jobs: Job[];
  builds: Build[];
  workers: Worker[];
  metrics: RuntimeMetrics | null;
  dead_letter_builds: Build[];
}

// Models the top-level GraphQL dashboard snapshot response.
export interface DashboardSnapshotResponse {
  dashboard_snapshot: DashboardSnapshot;
}

// Models the GraphQL create-job mutation response.
export interface CreateJobResponse {
  create_job: Pick<Job, "id" | "name">;
}

// Models the GraphQL run-job mutation response.
export interface RunJobResponse {
  run_job: Pick<Build, "id">;
}

// Models the GraphQL cancel-build mutation response.
export interface CancelBuildResponse {
  cancel_build: Pick<Build, "id">;
}

// Enumerates supported SCM providers for admin configuration forms.
export type ScmProvider = "github" | "gitlab";

// Represents SCM webhook security form state.
export interface WebhookSecurityInput {
  repository_url: string;
  provider: ScmProvider;
  secret: string;
  allowed_ips_text: string;
}

// Represents SCM polling administration form state.
export interface ScmPollingInput {
  repository_url: string;
  provider: ScmProvider;
  enabled: boolean;
  interval_secs_text: string;
  branches_text: string;
}

// Represents the response payload for one manual SCM polling tick.
export interface ScmPollingTickSummary {
  polled_repositories: number;
  enqueued_builds: number;
}

// Models the worker list API response.
export interface ListWorkersResponse {
  workers: Worker[];
}

// Models the worker claim-build API response.
export interface ClaimBuildResponse {
  build: Build | null;
}

// Models the worker complete-build API response.
export interface CompleteBuildResponse {
  build: Build;
}

// Represents one plugin entry shown in administration screens.
export interface PluginInfo {
  name: string;
  state: string;
  capabilities: string[];
  source_manifest_entry: string;
}

// Models the plugin inventory API response.
export interface ListPluginsResponse {
  plugins: PluginInfo[];
}

// Models plugin lifecycle action responses.
export interface PluginActionResponse {
  status: string;
  plugin: PluginInfo;
}

// Represents plugin administration form state.
export interface PluginAdminInput {
  name: string;
  production_tagged_context: boolean;
}

// Represents plugin policy editing form state.
export interface PluginPolicyInput {
  context: string;
  granted_capabilities: string[];
}

// Models plugin policy fetch/save responses.
export interface PluginPolicyResponse {
  context: string;
  granted_capabilities: string[];
}

// Models plugin authorization dry-run responses.
export interface PluginAuthorizationCheckResponse {
  plugin_name: string;
  context: string;
  required_capabilities: string[];
  granted_capabilities: string[];
  missing_capabilities: string[];
  allowed: boolean;
}

// Represents one SCM webhook rejection diagnostic entry.
export interface ScmWebhookRejectionEntry {
  reason_code: string;
  provider?: string;
  repository_url?: string;
  at: string;
}

// Models the SCM webhook rejection diagnostics response.
export interface ListScmWebhookRejectionsResponse {
  rejections: ScmWebhookRejectionEntry[];
}

// Models webhook-related counters returned by the dashboard GraphQL diagnostics query.
export interface RuntimeMetricsApiResponse {
  scm_webhook_received_total: number;
  scm_webhook_accepted_total: number;
  scm_webhook_rejected_total: number;
  scm_webhook_duplicate_total: number;
}

// Models the GraphQL mutation response for webhook security upsert.
export interface UpsertWebhookSecurityConfigResponse {
  upsert_webhook_security_config: boolean;
}

// Models the GraphQL mutation response for SCM polling configuration upsert.
export interface UpsertScmPollingConfigResponse {
  upsert_scm_polling_config: boolean;
}

// Models the GraphQL mutation response for one manual SCM polling tick.
export interface RunScmPollingTickResponse {
  run_scm_polling_tick: ScmPollingTickSummary;
}

// Models the GraphQL query response for webhook counters and diagnostics timeline.
export interface ScmWebhookOperationsResponse {
  metrics: RuntimeMetricsApiResponse;
  scm_webhook_rejections: ScmWebhookRejectionEntry[];
}

// Represents filters used by webhook operations diagnostics.
export interface ScmWebhookOpsFilter {
  provider: string;
  repository_url: string;
}

// Represents filters applied to observability event views.
export interface ObservabilityFilter {
  severity: string;
  kind: string;
  resource_id: string;
  window_minutes: string;
}

// Represents one in-memory audit entry associated with the current admin role.
export interface AdminActivityEntry {
  at: string;
  actor_role: AdminRole;
  action: string;
  target: string;
}

// Represents capability flags granted by one admin role.
export interface AdminRoleCapabilities {
  can_run_operations: boolean;
  can_mutate_sensitive: boolean;
}

// Models error payloads returned by REST administration endpoints.
export interface ApiErrorPayload {
  code?: string;
  message?: string;
}

// Enumerates worker completion status options for manual simulation controls.
export type WorkerCompletionStatus = "success" | "failed";

// Represents worker control form state.
export interface WorkerControlInput {
  worker_id: string;
  build_id: string;
  completion_status: WorkerCompletionStatus;
  completion_log_line: string;
}

// Represents the dashboard health state shown in the header.
export type HealthStatus = "ok" | "degraded";

// Represents the title/kicker pair displayed for one dashboard page.
export interface DashboardPagePresentation {
  kicker: string;
  title: string;
}

// Re-export shared core types so hook modules can import from a single source.
export type {
  AdminRole,
  Build,
  CreateJobInput,
  DashboardNavItem,
  DashboardPage,
  Job
};