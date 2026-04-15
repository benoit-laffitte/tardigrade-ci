// Shared widget-level model types used by dashboard view components.
export type BuildStatus = "Pending" | "Running" | "Success" | "Failed" | "Canceled";

// Enumerates dashboard pages available in the left-side navigation.
export type DashboardPage =
  | "pipelines"
  | "overview"
  | "workers"
  | "scm-security"
  | "plugins-policy"
  | "observability"
  | "administration";

// Enumerates currently supported role presets in the operations header.
export type AdminRole = "viewer" | "operator" | "admin";

// Represents one navigation tab with its API coverage badge.
export interface DashboardNavItem {
  id: DashboardPage;
  label: string;
  coverage: "full" | "partial" | "roadmap";
}

// Captures dashboard job metadata rendered in list widgets.
export interface Job {
  id: string;
  name: string;
  repository_url: string;
  pipeline_path: string;
  created_at: string;
}

// Captures dashboard build metadata rendered in list widgets.
export interface Build {
  id: string;
  job_id: string;
  status: BuildStatus;
  queued_at: string;
  started_at?: string | null;
  finished_at?: string | null;
  logs: string[];
}

// Represents create-job form state used by the pipelines widget.
export interface CreateJobInput {
  name: string;
  repository_url: string;
  pipeline_path: string;
}

// Represents the overview status counters shown in the breakdown panel.
export interface BuildStatusSummary {
  running: number;
  pending: number;
  success: number;
  failed: number;
  canceled: number;
}

// Represents derived execution pressure values for workers roadmap view.
export interface WorkersReadOnlySummary {
  running: number;
  pending: number;
  blockedRisk: number;
  pressure: string;
}

// Represents derived SCM risk values for SCM security roadmap view.
export interface ScmSecurityReadOnlySummary {
  failedBuilds: number;
  canceledBuilds: number;
  recentJobs: Job[];
  apiHealth: "ok" | "degraded";
}
