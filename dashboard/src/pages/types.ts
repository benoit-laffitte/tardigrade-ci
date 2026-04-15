import type {
  AdminActivityEntry,
  AdminRole,
  Build,
  BuildStatusSummary,
  CreateJobInput,
  Job,
  ScmSecurityReadOnlySummary,
  WorkersReadOnlySummary
} from "../widgets/types";
import type { Worker } from "../hooks/dashboardTypes";

// Defines cross-domain dependency props used to instantiate autonomous domain hooks.
interface DomainDeps {
  adminRole: AdminRole;
  roleCapabilities: {
    can_run_operations: boolean;
    can_mutate_sensitive: boolean;
  };
  log: (message: string, kind?: string) => void;
  audit: (action: string, target: string) => void;
}

// Defines props for the API-backed Pipelines page component.
export interface PipelinesPageProps {
  form: CreateJobInput;
  createMessage: string;
  jobs: Job[];
  builds: Build[];
  onCreateJob: (event: { preventDefault: () => void }) => void;
  onFormChange: (field: keyof CreateJobInput, value: string) => void;
  onRunJob: (jobId: string, name: string) => void;
  onCancelBuild: (buildId: string) => void;
  formatDateTime: (value?: string | null) => string;
}

// Defines props for the API-backed Overview page component.
export interface OverviewPageProps {
  jobs: Job[];
  builds: Build[];
  healthStatus: "ok" | "degraded";
  deliverySuccessRatio: string;
  buildStatusSummary: BuildStatusSummary;
  formatDateTime: (value?: string | null) => string;
}

// Defines props for the roadmap/read-only Workers page component.
export interface WorkersPageProps {
  workersReadOnlySummary: WorkersReadOnlySummary;
  recentExecutionBuilds: Build[];
  setWorkersSnapshot: (workers: Worker[]) => void;
  refreshAll: () => Promise<void>;
  adminRole: DomainDeps["adminRole"];
  roleCapabilities: DomainDeps["roleCapabilities"];
  log: DomainDeps["log"];
  audit: DomainDeps["audit"];
  formatDateTime: (value?: string | null) => string;
}

// Defines props for the roadmap/read-only SCM Security page component.
export interface ScmSecurityPageProps {
  scmSecurityReadOnlySummary: ScmSecurityReadOnlySummary;
  refreshAll: () => Promise<void>;
  adminRole: DomainDeps["adminRole"];
  roleCapabilities: DomainDeps["roleCapabilities"];
  log: DomainDeps["log"];
  audit: DomainDeps["audit"];
}

// Defines props for the roadmap Plugins & Policy page component.
export interface PluginsPolicyPageProps {
  adminRole: DomainDeps["adminRole"];
  roleCapabilities: DomainDeps["roleCapabilities"];
  log: DomainDeps["log"];
  audit: DomainDeps["audit"];
}

// Defines props for the roadmap Administration page component.
export interface AdministrationPageProps {
  adminActivity: AdminActivityEntry[];
  roleCapabilities: {
    can_run_operations: boolean;
    can_mutate_sensitive: boolean;
  };
}