import { gql } from "@apollo/client";

import type {
  AdminRole,
  AdminRoleCapabilities,
  DashboardNavItem,
  DashboardPage,
  DashboardPagePresentation
} from "./dashboardTypes";

// Lists supported plugin capability flags exposed in roadmap administration flows.
export const PLUGIN_CAPABILITY_OPTIONS = ["network", "filesystem", "secrets", "runtime_hooks"];

// Maps admin roles to the operations/sensitive capabilities they unlock.
export const ADMIN_ROLE_CAPABILITIES: Record<AdminRole, AdminRoleCapabilities> = {
  viewer: {
    can_run_operations: false,
    can_mutate_sensitive: false
  },
  operator: {
    can_run_operations: true,
    can_mutate_sensitive: false
  },
  admin: {
    can_run_operations: true,
    can_mutate_sensitive: true
  }
};

// Defines the 7 sidenav entries and their API coverage labels.
export const DASHBOARD_NAV_ITEMS: DashboardNavItem[] = [
  { id: "pipelines", label: "Pipelines", coverage: "full" },
  { id: "overview", label: "Overview", coverage: "partial" },
  { id: "workers", label: "Workers", coverage: "roadmap" },
  { id: "scm-security", label: "SCM Security", coverage: "roadmap" },
  { id: "plugins-policy", label: "Plugins & Policy", coverage: "roadmap" },
  { id: "observability", label: "Observability", coverage: "roadmap" },
  { id: "administration", label: "Administration", coverage: "roadmap" }
];

// Defines the title/kicker presentation metadata for each dashboard page.
export const PAGE_PRESENTATION: Record<DashboardPage, DashboardPagePresentation> = {
  pipelines: { kicker: "Delivery", title: "Pipelines" },
  overview: { kicker: "System Health", title: "Overview" },
  workers: { kicker: "Execution Plane", title: "Workers" },
  "scm-security": { kicker: "Trust Boundary", title: "SCM Security" },
  "plugins-policy": { kicker: "Governed Extensibility", title: "Plugins & Policy" },
  observability: { kicker: "Evidence", title: "Observability" },
  administration: { kicker: "Governance", title: "Administration" }
};

// Declares the GraphQL snapshot query used to populate the dashboard baseline data.
export const DASHBOARD_SNAPSHOT_QUERY = gql`
  query DashboardSnapshot {
    dashboard_snapshot {
      jobs {
        id
        name
        repository_url
        pipeline_path
        created_at
      }
      builds {
        id
        job_id
        status
        queued_at
        started_at
        finished_at
        logs
      }
      workers {
        id
        active_builds
        status
        last_seen_at
      }
      metrics {
        reclaimed_total
        retry_requeued_total
        ownership_conflicts_total
        dead_letter_total
      }
      dead_letter_builds {
        id
        job_id
        status
        queued_at
      }
    }
  }
`;

// Declares the GraphQL mutation used to create dashboard jobs.
export const CREATE_JOB_MUTATION = gql`
  mutation CreateJob($input: GqlCreateJobInput!) {
    create_job(input: $input) {
      id
      name
    }
  }
`;

// Declares the GraphQL mutation used to run one job immediately.
export const RUN_JOB_MUTATION = gql`
  mutation RunJob($jobId: ID!) {
    run_job(jobId: $jobId) {
      id
    }
  }
`;

// Declares the GraphQL mutation used to cancel one build.
export const CANCEL_BUILD_MUTATION = gql`
  mutation CancelBuild($buildId: ID!) {
    cancel_build(buildId: $buildId) {
      id
    }
  }
`;