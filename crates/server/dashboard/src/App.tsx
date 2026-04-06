import { gql, useApolloClient } from "@apollo/client";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type BuildStatus = "Pending" | "Running" | "Success" | "Failed" | "Canceled";
type EventSeverity = "ok" | "error" | "warn" | "info";

interface Job {
  id: string;
  name: string;
  repository_url: string;
  pipeline_path: string;
  created_at: string;
}

interface Build {
  id: string;
  job_id: string;
  status: BuildStatus;
  queued_at: string;
  started_at?: string | null;
  finished_at?: string | null;
  logs: string[];
}

interface Worker {
  id: string;
  active_builds: number;
  status: string;
  last_seen_at: string;
}

interface RuntimeMetrics {
  reclaimed_total: number;
  retry_requeued_total: number;
  ownership_conflicts_total: number;
  dead_letter_total: number;
}

interface LiveEvent {
  kind?: string;
  message?: string;
  severity?: EventSeverity;
  job_id?: string;
  build_id?: string;
  worker_id?: string;
  at?: string;
}

interface DashboardSnapshot {
  jobs: Job[];
  builds: Build[];
  workers: Worker[];
  metrics: RuntimeMetrics | null;
  dead_letter_builds: Build[];
}

interface DashboardSnapshotResponse {
  dashboard_snapshot: DashboardSnapshot;
}

interface CreateJobResponse {
  create_job: Pick<Job, "id" | "name">;
}

interface RunJobResponse {
  run_job: Pick<Build, "id">;
}

interface CancelBuildResponse {
  cancel_build: Pick<Build, "id">;
}

interface CreateJobInput {
  name: string;
  repository_url: string;
  pipeline_path: string;
}

type ScmProvider = "github" | "gitlab";

interface WebhookSecurityInput {
  repository_url: string;
  provider: ScmProvider;
  secret: string;
  allowed_ips_text: string;
}

interface ScmPollingInput {
  repository_url: string;
  provider: ScmProvider;
  enabled: boolean;
  interval_secs_text: string;
  branches_text: string;
}

interface ScmPollingTickSummary {
  polled_repositories: number;
  enqueued_builds: number;
}

interface ListWorkersResponse {
  workers: Worker[];
}

interface ClaimBuildResponse {
  build: Build | null;
}

interface CompleteBuildResponse {
  build: Build;
}

interface PluginInfo {
  name: string;
  state: string;
  capabilities: string[];
  source_manifest_entry: string;
}

interface ListPluginsResponse {
  plugins: PluginInfo[];
}

interface PluginActionResponse {
  status: string;
  plugin: PluginInfo;
}

interface PluginAdminInput {
  name: string;
  production_tagged_context: boolean;
}

interface PluginPolicyInput {
  context: string;
  granted_capabilities: string[];
}

interface PluginPolicyResponse {
  context: string;
  granted_capabilities: string[];
}

interface PluginAuthorizationCheckResponse {
  plugin_name: string;
  context: string;
  required_capabilities: string[];
  granted_capabilities: string[];
  missing_capabilities: string[];
  allowed: boolean;
}

interface ScmWebhookRejectionEntry {
  reason_code: string;
  provider?: string;
  repository_url?: string;
  at: string;
}

interface ListScmWebhookRejectionsResponse {
  rejections: ScmWebhookRejectionEntry[];
}

interface RuntimeMetricsApiResponse {
  scm_webhook_received_total: number;
  scm_webhook_accepted_total: number;
  scm_webhook_rejected_total: number;
  scm_webhook_duplicate_total: number;
}

interface ScmWebhookOpsFilter {
  provider: string;
  repository_url: string;
}

interface ObservabilityFilter {
  severity: string;
  kind: string;
  resource_id: string;
  window_minutes: string;
}

type AdminRole = "viewer" | "operator" | "admin";

type DashboardPage =
  | "pipelines"
  | "overview"
  | "workers"
  | "scm-security"
  | "plugins-policy"
  | "observability"
  | "administration";

interface DashboardNavItem {
  id: DashboardPage;
  label: string;
  coverage: "full" | "partial" | "roadmap";
}

interface AdminActivityEntry {
  at: string;
  actor_role: AdminRole;
  action: string;
  target: string;
}

interface AdminRoleCapabilities {
  can_run_operations: boolean;
  can_mutate_sensitive: boolean;
}

interface ApiErrorPayload {
  code?: string;
  message?: string;
}

const PLUGIN_CAPABILITY_OPTIONS = ["network", "filesystem", "secrets", "runtime_hooks"];

const ADMIN_ROLE_CAPABILITIES: Record<AdminRole, AdminRoleCapabilities> = {
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

const DASHBOARD_NAV_ITEMS: DashboardNavItem[] = [
  { id: "pipelines", label: "Pipelines", coverage: "full" },
  { id: "overview", label: "Overview", coverage: "partial" },
  { id: "workers", label: "Workers", coverage: "roadmap" },
  { id: "scm-security", label: "SCM Security", coverage: "roadmap" },
  { id: "plugins-policy", label: "Plugins & Policy", coverage: "roadmap" },
  { id: "observability", label: "Observability", coverage: "roadmap" },
  { id: "administration", label: "Administration", coverage: "roadmap" }
];

type WorkerCompletionStatus = "success" | "failed";

interface WorkerControlInput {
  worker_id: string;
  build_id: string;
  completion_status: WorkerCompletionStatus;
  completion_log_line: string;
}

const DASHBOARD_SNAPSHOT_QUERY = gql`
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

const CREATE_JOB_MUTATION = gql`
  mutation CreateJob($input: GqlCreateJobInput!) {
    create_job(input: $input) {
      id
      name
    }
  }
`;

const RUN_JOB_MUTATION = gql`
  mutation RunJob($jobId: ID!) {
    run_job(jobId: $jobId) {
      id
    }
  }
`;

const CANCEL_BUILD_MUTATION = gql`
  mutation CancelBuild($buildId: ID!) {
    cancel_build(buildId: $buildId) {
      id
    }
  }
`;

// Maps incoming SSE severity values to dashboard badge classes.
function severityToStatusClass(severity?: EventSeverity): "success" | "failed" | "pending" {
  if (severity === "ok") {
    return "success";
  }
  if (severity === "error") {
    return "failed";
  }
  return "pending";
}

// Formats timestamps in local time while handling missing values.
function formatDateTime(value?: string | null): string {
  if (!value) {
    return "-";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "-";
  }
  return date.toLocaleString();
}

// Formats timestamps in local time (short variant for event feed).
function formatTime(value?: string | null): string {
  if (!value) {
    return new Date().toLocaleTimeString();
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return new Date().toLocaleTimeString();
  }
  return date.toLocaleTimeString();
}

// Normalizes comma/newline-delimited text into unique trimmed entries.
function normalizeDelimitedInput(raw: string): string[] {
  const values = raw
    .split(/[,\n]/)
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  return Array.from(new Set(values));
}

// Normalizes allowlist text input into unique trimmed IP entries.
function normalizeAllowlistInput(raw: string): string[] {
  return normalizeDelimitedInput(raw);
}

// Normalizes branch text input into unique trimmed branch names.
function normalizeBranchesInput(raw: string): string[] {
  return normalizeDelimitedInput(raw);
}

// Computes missing capabilities from required/granted sets for policy explainability.
function missingCapabilities(required: string[], granted: string[]): string[] {
  return required.filter((capability) => !granted.includes(capability));
}

// Converts event rows to CSV text for incident handoff exports.
function observabilityEventsToCsv(events: LiveEvent[]): string {
  const header = ["at", "kind", "severity", "message", "job_id", "build_id", "worker_id"];
  const escape = (value?: string) => `"${(value ?? "").replaceAll("\"", "\"\"")}"`;
  const rows = events.map((event) => [
    event.at ?? "",
    event.kind ?? "",
    event.severity ?? "",
    event.message ?? "",
    event.job_id ?? "",
    event.build_id ?? "",
    event.worker_id ?? ""
  ]);

  return [header, ...rows].map((row) => row.map((value) => escape(String(value))).join(",")).join("\n");
}

// Triggers browser download for one text payload using provided mime type.
function downloadTextPayload(filename: string, content: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

// Checks whether filter resource id matches any event resource identifier.
function matchesEventResource(event: LiveEvent, resourceId: string): boolean {
  if (!resourceId) {
    return true;
  }
  const resource = `${event.job_id ?? ""} ${event.build_id ?? ""} ${event.worker_id ?? ""}`.toLowerCase();
  return resource.includes(resourceId);
}

// Checks whether one event falls inside configured observability time window.
function matchesEventWindow(event: LiveEvent, windowMinutes: number, nowMs: number): boolean {
  if (!(Number.isFinite(windowMinutes) && windowMinutes > 0 && windowMinutes < 100000)) {
    return true;
  }

  const eventTs = event.at ? new Date(event.at).getTime() : nowMs;
  if (Number.isNaN(eventTs)) {
    return true;
  }

  const ageMinutes = (nowMs - eventTs) / 60000;
  return ageMinutes <= windowMinutes;
}

// Applies one observability filter set to one live event.
function matchesObservabilityFilter(
  event: LiveEvent,
  filter: ObservabilityFilter,
  nowMs: number
): boolean {
  const severity = filter.severity.trim().toLowerCase();
  const kind = filter.kind.trim().toLowerCase();
  const resourceId = filter.resource_id.trim().toLowerCase();
  const windowMinutes = Number.parseInt(filter.window_minutes, 10);

  if (severity && String(event.severity ?? "").toLowerCase() !== severity) {
    return false;
  }

  if (kind && !String(event.kind ?? "").toLowerCase().includes(kind)) {
    return false;
  }

  if (!matchesEventResource(event, resourceId)) {
    return false;
  }

  if (!matchesEventWindow(event, windowMinutes, nowMs)) {
    return false;
  }

  return true;
}

// Returns the display stardate used in the top HUD strip.
function stardateValue(now: Date): string {
  const yearStart = new Date(now.getFullYear(), 0, 1);
  const dayOfYear = Math.floor((now.getTime() - yearStart.getTime()) / 86400000) + 1;
  return `${String(now.getFullYear()).slice(2)}.${String(dayOfYear).padStart(3, "0")}`;
}

// Keeps transitional variables referenced while roadmap pages are progressively implemented.
function keepRoadmapReferences(..._args: unknown[]): void {
  return;
}

// NOSONAR - transitional component during multi-page migration from mockup to real API-backed views.
// eslint-disable-next-line sonarjs/cognitive-complexity
export function App() {
  const client = useApolloClient();
  const refreshTimerRef = useRef<number | null>(null);

  const [streamConnected, setStreamConnected] = useState(false);
  const [activePage, setActivePage] = useState<DashboardPage>("pipelines");
  const [healthStatus, setHealthStatus] = useState<"ok" | "degraded">("degraded");
  const [snapshot, setSnapshot] = useState<DashboardSnapshot>({
    jobs: [],
    builds: [],
    workers: [],
    metrics: null,
    dead_letter_builds: []
  });
  const [liveEvents, setLiveEvents] = useState<LiveEvent[]>([]);
  const [adminRole, setAdminRole] = useState<AdminRole>("admin");
  const [adminActivity, setAdminActivity] = useState<AdminActivityEntry[]>([]);
  const [logs, setLogs] = useState("");
  const [createMessage, setCreateMessage] = useState("");
  const [form, setForm] = useState<CreateJobInput>({
    name: "",
    repository_url: "",
    pipeline_path: ""
  });
  const [webhookForm, setWebhookForm] = useState<WebhookSecurityInput>({
    repository_url: "",
    provider: "github",
    secret: "",
    allowed_ips_text: ""
  });
  const [webhookMessage, setWebhookMessage] = useState("");
  const [showWebhookSecret, setShowWebhookSecret] = useState(false);
  const [knownWebhookConfigs, setKnownWebhookConfigs] = useState<Set<string>>(new Set());
  const [pollingForm, setPollingForm] = useState<ScmPollingInput>({
    repository_url: "",
    provider: "github",
    enabled: true,
    interval_secs_text: "30",
    branches_text: "main"
  });
  const [pollingMessage, setPollingMessage] = useState("");
  const [pollingTickSummary, setPollingTickSummary] = useState<ScmPollingTickSummary | null>(null);
  const [knownPollingStates, setKnownPollingStates] = useState<Map<string, boolean>>(new Map());
  const [workerControlForm, setWorkerControlForm] = useState<WorkerControlInput>({
    worker_id: "",
    build_id: "",
    completion_status: "success",
    completion_log_line: ""
  });
  const [workerControlMessage, setWorkerControlMessage] = useState("");
  const [lastClaimResult, setLastClaimResult] = useState<string>("");
  const [pluginAdminForm, setPluginAdminForm] = useState<PluginAdminInput>({
    name: "",
    production_tagged_context: false
  });
  const [pluginAdminMessage, setPluginAdminMessage] = useState("");
  const [pluginInventory, setPluginInventory] = useState<PluginInfo[]>([]);
  const [pluginPolicyForm, setPluginPolicyForm] = useState<PluginPolicyInput>({
    context: "global",
    granted_capabilities: []
  });
  const [pluginPolicyMessage, setPluginPolicyMessage] = useState("");
  const [pluginAuthorizationResult, setPluginAuthorizationResult] =
    useState<PluginAuthorizationCheckResponse | null>(null);
  const [effectivePolicyContext, setEffectivePolicyContext] = useState("global");
  const [effectiveGrantedCapabilities, setEffectiveGrantedCapabilities] = useState<string[]>([]);
  const [scmWebhookOpsFilter, setScmWebhookOpsFilter] = useState<ScmWebhookOpsFilter>({
    provider: "",
    repository_url: ""
  });
  const [scmWebhookOpsMessage, setScmWebhookOpsMessage] = useState("");
  const [scmWebhookMetrics, setScmWebhookMetrics] = useState<RuntimeMetricsApiResponse | null>(null);
  const [scmWebhookRejections, setScmWebhookRejections] = useState<ScmWebhookRejectionEntry[]>([]);
  const [observabilityFilter, setObservabilityFilter] = useState<ObservabilityFilter>({
    severity: "",
    kind: "",
    resource_id: "",
    window_minutes: "15"
  });
  const [observabilityMessage, setObservabilityMessage] = useState("");
  const [stardate, setStardate] = useState(() => stardateValue(new Date()));

  // Resolves API coverage label for currently selected page in the navigation bar.
  const activeCoverage = useMemo(() => {
    const current = DASHBOARD_NAV_ITEMS.find((item) => item.id === activePage);
    return current?.coverage ?? "roadmap";
  }, [activePage]);

  // Identifies pages already wired to the currently available API surface.
  const isImplementedPage = activePage === "pipelines" || activePage === "overview";

  // Prepends one log line to keep operator feedback visible.
  const log = useCallback((message: string, kind: string = "info") => {
    const now = new Date().toLocaleTimeString();
    const prefix = kind.toUpperCase().padEnd(5, " ");
    setLogs((prev) => `[${now}] ${prefix} ${message}\n${prev}`);
  }, []);

  // Appends one local audit entry for admin actions and role-gated attempts.
  const audit = useCallback((action: string, target: string) => {
    const at = new Date().toISOString();
    setAdminActivity((previous) => [{ at, actor_role: adminRole, action, target }, ...previous].slice(0, 40));
  }, [adminRole]);

  // Checks whether current role grants operation/sensitive mutation capability.
  const roleCapabilities = useMemo(() => ADMIN_ROLE_CAPABILITIES[adminRole], [adminRole]);

  // Pulls snapshot data from GraphQL and updates all dashboard panels.
  const refreshAll = useCallback(async () => {
    try {
      const { data } = await client.query<DashboardSnapshotResponse>({
        query: DASHBOARD_SNAPSHOT_QUERY,
        fetchPolicy: "network-only"
      });
      setSnapshot(data.dashboard_snapshot);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      log(`Echec du rafraichissement: ${message}`, "error");
    }
  }, [client, log]);

  // Debounces refresh calls to avoid overloading API on event bursts.
  const scheduleRefresh = useCallback(
    (delayMs: number = 120) => {
      if (refreshTimerRef.current) {
        globalThis.clearTimeout(refreshTimerRef.current);
      }
      refreshTimerRef.current = globalThis.setTimeout(() => {
        refreshTimerRef.current = null;
        void refreshAll();
      }, delayMs);
    },
    [refreshAll]
  );

  // Stores one event in memory, rerenders feed, and writes an operator log.
  const pushLiveEvent = useCallback(
    (evt: LiveEvent) => {
      setLiveEvents((previous) => {
        const next = [evt, ...previous];
        return next.slice(0, 30);
      });
      log(`${evt.kind ?? "event"}: ${evt.message ?? "update"}`, evt.severity ?? "info");
    },
    [log]
  );

  // Executes one job and refreshes the dashboard after mutation completes.
  const runJob = useCallback(
    async (jobId: string, name: string) => {
      if (!roleCapabilities.can_run_operations) {
        log(`Role ${adminRole} ne peut pas lancer de build`, "warn");
        audit("run_job_denied", name);
        return;
      }

      try {
        const { data } = await client.mutate<RunJobResponse>({
          mutation: RUN_JOB_MUTATION,
          variables: { jobId }
        });
        if (!data?.run_job.id) {
          throw new Error("run_job did not return a build id");
        }
        log(`Build ${data.run_job.id.slice(0, 8)} lance pour ${name}`, "ok");
        audit("run_job", name);
        await refreshAll();
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        log(`Impossible de lancer le job ${name}: ${message}`, "error");
      }
    },
    [adminRole, audit, client, log, refreshAll, roleCapabilities.can_run_operations]
  );

  // Cancels one build and refreshes dashboard state.
  const cancelBuild = useCallback(
    async (buildId: string) => {
      if (!roleCapabilities.can_run_operations) {
        log(`Role ${adminRole} ne peut pas annuler de build`, "warn");
        audit("cancel_build_denied", buildId);
        return;
      }

      try {
        await client.mutate<CancelBuildResponse>({
          mutation: CANCEL_BUILD_MUTATION,
          variables: { buildId }
        });
        log(`Build ${buildId.slice(0, 8)} annule`, "ok");
        audit("cancel_build", buildId);
        await refreshAll();
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        log(`Impossible d'annuler ${buildId.slice(0, 8)}: ${message}`, "error");
      }
    },
    [adminRole, audit, client, log, refreshAll, roleCapabilities.can_run_operations]
  );

  // Creates a job from form payload and refreshes dashboard data.
  const createJob = useCallback(
    async (event: { preventDefault: () => void }) => {
      event.preventDefault();
      if (!roleCapabilities.can_run_operations) {
        setCreateMessage("Role insuffisant pour creer un job.");
        log(`Role ${adminRole} ne peut pas creer de job`, "warn");
        audit("create_job_denied", form.name || "unknown");
        return;
      }

      setCreateMessage("Creation en cours...");

      try {
        const { data } = await client.mutate<CreateJobResponse>({
          mutation: CREATE_JOB_MUTATION,
          variables: { input: form }
        });

        if (!data?.create_job) {
          throw new Error("create_job did not return payload");
        }

        setCreateMessage(`Job ${data.create_job.name} cree.`);
        log(`Nouveau job ${data.create_job.name} (${data.create_job.id.slice(0, 8)})`, "ok");
        audit("create_job", data.create_job.name);
        setForm({ name: "", repository_url: "", pipeline_path: "" });
        await refreshAll();
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        setCreateMessage("Erreur de creation");
        log(`Creation job en erreur: ${message}`, "error");
      }
    },
    [adminRole, audit, client, form, log, refreshAll, roleCapabilities.can_run_operations]
  );

  // Saves webhook security settings for one repository/provider pair.
  const saveWebhookSecurityConfig = useCallback(
    async (event: { preventDefault: () => void }) => {
      event.preventDefault();
      if (!roleCapabilities.can_mutate_sensitive) {
        setWebhookMessage("Role insuffisant pour modifier la securite webhook.");
        log(`Role ${adminRole} ne peut pas modifier webhook security`, "warn");
        audit("webhook_security_update_denied", webhookForm.repository_url || "unknown");
        return;
      }

      const repository = webhookForm.repository_url.trim();
      const secret = webhookForm.secret.trim();
      const configKey = `${repository.toLowerCase()}::${webhookForm.provider}`;

      if (!repository || !secret) {
        setWebhookMessage("Parametres invalides: repository et secret requis.");
        log("Configuration webhook invalide: repository/secret manquant", "warn");
        return;
      }

      if (knownWebhookConfigs.has(configKey)) {
        const confirmed = globalThis.confirm(
          "Une configuration existe deja pour ce repository/provider. Confirmer l'ecrasement ?"
        );
        if (!confirmed) {
          setWebhookMessage("Ecrasement annule.");
          return;
        }
      }

      const payload = {
        repository_url: repository,
        provider: webhookForm.provider,
        secret,
        allowed_ips: normalizeAllowlistInput(webhookForm.allowed_ips_text)
      };

      try {
        const response = await fetch("/scm/webhook-security/configs", {
          method: "POST",
          headers: {
            "content-type": "application/json"
          },
          body: JSON.stringify(payload)
        });

        if (response.status === 204) {
          setWebhookMessage("Configuration webhook enregistree.");
          setKnownWebhookConfigs((previous) => new Set(previous).add(configKey));
          log(`Webhook security sauvegardee pour ${repository} (${webhookForm.provider})`, "ok");
          audit("webhook_security_update", repository);
          return;
        }

        if (response.status === 400) {
          setWebhookMessage("Configuration invalide.");
          log("Rejet de configuration webhook: payload invalide", "warn");
          return;
        }

        if (response.status === 403) {
          setWebhookMessage("Configuration refusee (forbidden).");
          log("Configuration webhook refusee (403)", "error");
          return;
        }

        setWebhookMessage("Erreur interne lors de la sauvegarde webhook.");
        log(`Configuration webhook en echec: HTTP ${response.status}`, "error");
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        setWebhookMessage("Erreur reseau lors de la sauvegarde webhook.");
        log(`Configuration webhook en echec: ${message}`, "error");
      }
    },
    [adminRole, audit, knownWebhookConfigs, log, roleCapabilities.can_mutate_sensitive, webhookForm]
  );

  // Saves one SCM polling configuration for repository/provider pair.
  const saveScmPollingConfig = useCallback(
    async (event: { preventDefault: () => void }) => {
      event.preventDefault();
      if (!roleCapabilities.can_mutate_sensitive) {
        setPollingMessage("Role insuffisant pour modifier la configuration polling.");
        log(`Role ${adminRole} ne peut pas modifier polling config`, "warn");
        audit("polling_config_update_denied", pollingForm.repository_url || "unknown");
        return;
      }

      const repository = pollingForm.repository_url.trim();
      const configKey = `${repository.toLowerCase()}::${pollingForm.provider}`;
      const interval = Number.parseInt(pollingForm.interval_secs_text, 10);

      if (!repository || !Number.isFinite(interval) || interval <= 0) {
        setPollingMessage("Parametres invalides: repository requis et interval > 0.");
        log("Configuration polling invalide", "warn");
        return;
      }

      if (knownPollingStates.get(configKey) === true && !pollingForm.enabled) {
        const confirmed = globalThis.confirm(
          "Confirmer la desactivation de ce polling repository/provider ?"
        );
        if (!confirmed) {
          setPollingMessage("Desactivation annulee.");
          return;
        }
      }

      const payload = {
        repository_url: repository,
        provider: pollingForm.provider,
        enabled: pollingForm.enabled,
        interval_secs: interval,
        branches: normalizeBranchesInput(pollingForm.branches_text)
      };

      try {
        const response = await fetch("/scm/polling/configs", {
          method: "POST",
          headers: {
            "content-type": "application/json"
          },
          body: JSON.stringify(payload)
        });

        if (response.status === 204) {
          setKnownPollingStates((previous) => {
            const next = new Map(previous);
            next.set(configKey, pollingForm.enabled);
            return next;
          });
          setPollingMessage(
            pollingForm.enabled ? "Configuration polling enregistree." : "Polling desactive."
          );
          log(`Polling config sauvegardee pour ${repository} (${pollingForm.provider})`, "ok");
          audit("polling_config_update", repository);
          return;
        }

        if (response.status === 400) {
          setPollingMessage("Configuration polling invalide.");
          log("Rejet configuration polling (400)", "warn");
          return;
        }

        setPollingMessage("Erreur interne lors de la sauvegarde polling.");
        log(`Configuration polling en echec: HTTP ${response.status}`, "error");
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        setPollingMessage("Erreur reseau lors de la sauvegarde polling.");
        log(`Configuration polling en echec: ${message}`, "error");
      }
    },
    [
      adminRole,
      audit,
      knownPollingStates,
      log,
      pollingForm,
      roleCapabilities.can_mutate_sensitive
    ]
  );

  // Triggers one manual SCM polling tick and renders immediate summary.
  const runManualScmPollingTick = useCallback(async () => {
    try {
      const response = await fetch("/scm/polling/tick", {
        method: "POST"
      });

      if (!response.ok) {
        setPollingMessage(`Tick polling en echec (HTTP ${response.status}).`);
        log(`Tick polling en echec: HTTP ${response.status}`, "error");
        return;
      }

      const payload = (await response.json()) as ScmPollingTickSummary;
      setPollingTickSummary(payload);
      setPollingMessage(
        `Tick execute: ${payload.polled_repositories} repo(s), ${payload.enqueued_builds} build(s) enqueued.`
      );
      log(
        `Tick polling: repos=${payload.polled_repositories}, enqueued=${payload.enqueued_builds}`,
        "ok"
      );
      await refreshAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPollingMessage("Erreur reseau lors du tick polling.");
      log(`Tick polling en echec: ${message}`, "error");
    }
  }, [log, refreshAll]);

  // Reads one API error payload and extracts actionable message for operator feedback.
  const parseApiErrorMessage = useCallback(async (response: Response): Promise<string> => {
    try {
      const payload = (await response.json()) as ApiErrorPayload;
      return payload.message ?? `HTTP ${response.status}`;
    } catch {
      return `HTTP ${response.status}`;
    }
  }, []);

  // Refreshes plugin registry inventory for administration panel.
  const refreshPluginInventory = useCallback(async () => {
    try {
      const response = await fetch("/plugins", { method: "GET" });
      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Chargement plugins en echec: ${details}.`);
        log(`Chargement plugins en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as ListPluginsResponse;
      setPluginInventory(payload.plugins);
      setPluginAdminMessage(`Inventaire plugins rafraichi (${payload.plugins.length}).`);
      log(`Inventaire plugins rafraichi (${payload.plugins.length})`, "ok");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors du chargement plugins.");
      log(`Chargement plugins en echec: ${message}`, "error");
    }
  }, [log, parseApiErrorMessage]);

  // Loads one plugin from built-in server catalog.
  const loadPlugin = useCallback(async () => {
    if (!roleCapabilities.can_mutate_sensitive) {
      setPluginAdminMessage("Role insuffisant pour charger un plugin.");
      log(`Role ${adminRole} ne peut pas charger de plugin`, "warn");
      audit("plugin_load_denied", pluginAdminForm.name || "unknown");
      return;
    }

    const name = pluginAdminForm.name.trim();
    if (!name) {
      setPluginAdminMessage("Nom plugin requis.");
      log("Load plugin refuse: nom manquant", "warn");
      return;
    }

    try {
      const response = await fetch("/plugins", {
        method: "POST",
        headers: {
          "content-type": "application/json"
        },
        body: JSON.stringify({ name })
      });

      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Load plugin en echec: ${details}.`);
        log(`Load plugin ${name} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginActionResponse;
      setPluginAdminMessage(`Plugin ${payload.plugin.name} ${payload.status}.`);
      log(`Plugin ${payload.plugin.name} ${payload.status}`, "ok");
      audit("plugin_load", payload.plugin.name);
      await refreshPluginInventory();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors du chargement plugin.");
      log(`Load plugin ${name} en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginAdminForm.name,
    refreshPluginInventory,
    roleCapabilities.can_mutate_sensitive
  ]);

  // Initializes one already loaded plugin.
  const initPlugin = useCallback(async () => {
    if (!roleCapabilities.can_run_operations) {
      setPluginAdminMessage("Role insuffisant pour initialiser un plugin.");
      log(`Role ${adminRole} ne peut pas initialiser de plugin`, "warn");
      audit("plugin_init_denied", pluginAdminForm.name || "unknown");
      return;
    }

    const name = pluginAdminForm.name.trim();
    if (!name) {
      setPluginAdminMessage("Nom plugin requis.");
      log("Init plugin refuse: nom manquant", "warn");
      return;
    }

    try {
      const response = await fetch(`/plugins/${encodeURIComponent(name)}/init`, {
        method: "POST"
      });
      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Init plugin en echec: ${details}.`);
        log(`Init plugin ${name} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginActionResponse;
      setPluginAdminMessage(`Plugin ${payload.plugin.name} ${payload.status}.`);
      log(`Plugin ${payload.plugin.name} ${payload.status}`, "ok");
      audit("plugin_init", payload.plugin.name);
      await refreshPluginInventory();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors de l'initialisation plugin.");
      log(`Init plugin ${name} en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginAdminForm.name,
    refreshPluginInventory,
    roleCapabilities.can_run_operations
  ]);

  // Executes one plugin, requiring confirmation when context is production tagged.
  const executePlugin = useCallback(async () => {
    if (!roleCapabilities.can_run_operations) {
      setPluginAdminMessage("Role insuffisant pour executer un plugin.");
      log(`Role ${adminRole} ne peut pas executer de plugin`, "warn");
      audit("plugin_execute_denied", pluginAdminForm.name || "unknown");
      return;
    }

    const name = pluginAdminForm.name.trim();
    if (!name) {
      setPluginAdminMessage("Nom plugin requis.");
      log("Execute plugin refuse: nom manquant", "warn");
      return;
    }

    if (pluginAdminForm.production_tagged_context) {
      const confirmed = globalThis.confirm(
        "Contexte tagge production: confirmer l'execution diagnostique du plugin ?"
      );
      if (!confirmed) {
        setPluginAdminMessage("Execution plugin annulee.");
        return;
      }
    }

    try {
      const response = await fetch(`/plugins/${encodeURIComponent(name)}/execute`, {
        method: "POST"
      });
      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Execute plugin en echec: ${details}.`);
        log(`Execute plugin ${name} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginActionResponse;
      setPluginAdminMessage(`Plugin ${payload.plugin.name} ${payload.status}.`);
      log(`Plugin ${payload.plugin.name} ${payload.status}`, "ok");
      audit("plugin_execute", payload.plugin.name);
      await refreshPluginInventory();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors de l'execution plugin.");
      log(`Execute plugin ${name} en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginAdminForm,
    refreshPluginInventory,
    roleCapabilities.can_run_operations
  ]);

  // Unloads one plugin after explicit operator confirmation.
  const unloadPlugin = useCallback(async () => {
    if (!roleCapabilities.can_mutate_sensitive) {
      setPluginAdminMessage("Role insuffisant pour decharger un plugin.");
      log(`Role ${adminRole} ne peut pas decharger de plugin`, "warn");
      audit("plugin_unload_denied", pluginAdminForm.name || "unknown");
      return;
    }

    const name = pluginAdminForm.name.trim();
    if (!name) {
      setPluginAdminMessage("Nom plugin requis.");
      log("Unload plugin refuse: nom manquant", "warn");
      return;
    }

    const confirmed = globalThis.confirm(`Confirmer le dechargement du plugin ${name} ?`);
    if (!confirmed) {
      setPluginAdminMessage("Dechargement plugin annule.");
      return;
    }

    try {
      const response = await fetch(`/plugins/${encodeURIComponent(name)}/unload`, {
        method: "POST"
      });
      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Unload plugin en echec: ${details}.`);
        log(`Unload plugin ${name} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginActionResponse;
      setPluginAdminMessage(`Plugin ${payload.plugin.name} ${payload.status}.`);
      log(`Plugin ${payload.plugin.name} ${payload.status}`, "ok");
      audit("plugin_unload", payload.plugin.name);
      await refreshPluginInventory();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors du dechargement plugin.");
      log(`Unload plugin ${name} en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginAdminForm.name,
    refreshPluginInventory,
    roleCapabilities.can_mutate_sensitive
  ]);

  // Toggles one capability in plugin policy form while preserving uniqueness.
  const togglePluginPolicyCapability = useCallback((capability: string, checked: boolean) => {
    setPluginPolicyForm((previous) => {
      const nextCapabilities = checked
        ? Array.from(new Set([...previous.granted_capabilities, capability]))
        : previous.granted_capabilities.filter((value) => value !== capability);
      return { ...previous, granted_capabilities: nextCapabilities };
    });
  }, []);

  // Loads effective policy values for selected context and syncs form toggles.
  const loadPluginPolicy = useCallback(async () => {
    const context = pluginPolicyForm.context.trim() || "global";
    try {
      const response = await fetch(`/plugins/policies?context=${encodeURIComponent(context)}`, {
        method: "GET"
      });

      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginPolicyMessage(`Chargement policy en echec: ${details}.`);
        log(`Chargement policy plugin en echec (${context}): ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginPolicyResponse;
      setEffectivePolicyContext(payload.context);
      setEffectiveGrantedCapabilities(payload.granted_capabilities);
      setPluginPolicyForm((previous) => ({
        ...previous,
        context: payload.context,
        granted_capabilities: payload.granted_capabilities
      }));
      setPluginPolicyMessage(
        `Policy chargee (${payload.context}): ${payload.granted_capabilities.join(", ") || "none"}.`
      );
      log(
        `Policy plugin chargee (${payload.context}) caps=${payload.granted_capabilities.join(",") || "none"}`,
        "ok"
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginPolicyMessage("Erreur reseau lors du chargement policy.");
      log(`Chargement policy plugin en echec (${context}): ${message}`, "error");
    }
  }, [log, parseApiErrorMessage, pluginPolicyForm.context]);

  // Saves granted capabilities for selected plugin execution context.
  const savePluginPolicy = useCallback(async () => {
    if (!roleCapabilities.can_mutate_sensitive) {
      setPluginPolicyMessage("Role insuffisant pour modifier la policy plugin.");
      log(`Role ${adminRole} ne peut pas modifier plugin policy`, "warn");
      audit("plugin_policy_update_denied", pluginPolicyForm.context || "global");
      return;
    }

    const context = pluginPolicyForm.context.trim() || "global";
    const wantsSecrets = pluginPolicyForm.granted_capabilities.includes("secrets");

    if (wantsSecrets) {
      const confirmed = globalThis.confirm(
        "Confirmer l'octroi de la capacite secrets pour ce contexte ?"
      );
      if (!confirmed) {
        setPluginPolicyMessage("Mise a jour policy annulee.");
        return;
      }
    }

    try {
      const response = await fetch("/plugins/policies", {
        method: "POST",
        headers: {
          "content-type": "application/json"
        },
        body: JSON.stringify({
          context,
          granted_capabilities: pluginPolicyForm.granted_capabilities
        })
      });

      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginPolicyMessage(`Policy en echec: ${details}.`);
        log(`Policy plugin en echec (${context}): ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginPolicyResponse;
      setEffectivePolicyContext(payload.context);
      setEffectiveGrantedCapabilities(payload.granted_capabilities);
      setPluginPolicyMessage(
        `Policy enregistree (${payload.context}): ${payload.granted_capabilities.join(", ") || "none"}.`
      );
      audit("plugin_policy_update", payload.context);
      log(
        `Policy plugin sauvegardee (${payload.context}) caps=${payload.granted_capabilities.join(",") || "none"}`,
        "ok"
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginPolicyMessage("Erreur reseau lors de la sauvegarde policy.");
      log(`Policy plugin en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginPolicyForm,
    roleCapabilities.can_mutate_sensitive
  ]);

  // Runs authorization dry-run for selected plugin and context, then renders allow/deny diff.
  const runPluginAuthorizationCheck = useCallback(async () => {
    const pluginName = pluginAdminForm.name.trim();
    const context = pluginPolicyForm.context.trim() || "global";

    if (!pluginName) {
      setPluginPolicyMessage("Nom plugin requis pour verification policy.");
      log("Authorize-check refuse: nom plugin manquant", "warn");
      return;
    }

    try {
      const response = await fetch(`/plugins/${encodeURIComponent(pluginName)}/authorize-check`, {
        method: "POST",
        headers: {
          "content-type": "application/json"
        },
        body: JSON.stringify({ context })
      });

      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginPolicyMessage(`Authorize-check en echec: ${details}.`);
        log(`Authorize-check plugin ${pluginName} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginAuthorizationCheckResponse;
      setPluginAuthorizationResult(payload);
      if (payload.allowed) {
        setPluginPolicyMessage(`Policy allow pour ${payload.plugin_name} (${payload.context}).`);
        log(`Policy allow ${payload.plugin_name} (${payload.context})`, "ok");
      } else {
        setPluginPolicyMessage(
          `Policy deny pour ${payload.plugin_name}: missing ${payload.missing_capabilities.join(", ")}.`
        );
        log(
          `Policy deny ${payload.plugin_name} (${payload.context}) missing=${payload.missing_capabilities.join(",")}`,
          "warn"
        );
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginPolicyMessage("Erreur reseau lors du dry-run policy.");
      log(`Authorize-check plugin ${pluginName} en echec: ${message}`, "error");
    }
  }, [log, parseApiErrorMessage, pluginAdminForm.name, pluginPolicyForm.context]);

  // Refreshes SCM webhook counters and recent rejection diagnostics timeline.
  const refreshScmWebhookOperations = useCallback(async () => {
    const provider = scmWebhookOpsFilter.provider.trim();
    const repositoryUrl = scmWebhookOpsFilter.repository_url.trim();

    const query = new URLSearchParams();
    if (provider) {
      query.set("provider", provider);
    }
    if (repositoryUrl) {
      query.set("repository_url", repositoryUrl);
    }
    query.set("limit", "20");

    try {
      const [metricsResponse, diagnosticsResponse] = await Promise.all([
        fetch("/metrics", { method: "GET" }),
        fetch(`/scm/webhook-security/rejections?${query.toString()}`, { method: "GET" })
      ]);

      if (!metricsResponse.ok) {
        const details = await parseApiErrorMessage(metricsResponse);
        setScmWebhookOpsMessage(`Chargement metrics webhook en echec: ${details}.`);
        log(`Metrics webhook en echec: ${details}`, "error");
        return;
      }

      if (!diagnosticsResponse.ok) {
        const details = await parseApiErrorMessage(diagnosticsResponse);
        setScmWebhookOpsMessage(`Chargement diagnostics webhook en echec: ${details}.`);
        log(`Diagnostics webhook en echec: ${details}`, "error");
        return;
      }

      const metricsPayload = (await metricsResponse.json()) as RuntimeMetricsApiResponse;
      const diagnosticsPayload = (await diagnosticsResponse.json()) as ListScmWebhookRejectionsResponse;
      setScmWebhookMetrics(metricsPayload);
      setScmWebhookRejections(diagnosticsPayload.rejections);
      setScmWebhookOpsMessage(
        `Diagnostics webhook rafraichis (${diagnosticsPayload.rejections.length} rejection(s)).`
      );
      log(
        `Diagnostics webhook rafraichis: received=${metricsPayload.scm_webhook_received_total}, rejected=${metricsPayload.scm_webhook_rejected_total}`,
        "ok"
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setScmWebhookOpsMessage("Erreur reseau lors du chargement webhook operations.");
      log(`Webhook operations en echec: ${message}`, "error");
    }
  }, [log, parseApiErrorMessage, scmWebhookOpsFilter.provider, scmWebhookOpsFilter.repository_url]);

  // Fetches worker list from worker API and updates worker panel state.
  const refreshWorkers = useCallback(async () => {
    try {
      const response = await fetch("/workers", { method: "GET" });
      if (!response.ok) {
        setWorkerControlMessage(`Impossible de charger les workers (HTTP ${response.status}).`);
        log(`Chargement workers en echec: HTTP ${response.status}`, "error");
        return;
      }

      const payload = (await response.json()) as ListWorkersResponse;
      setSnapshot((previous) => ({ ...previous, workers: payload.workers }));
      setWorkerControlMessage(`Workers rafraichis: ${payload.workers.length}.`);
      log(`Workers rafraichis (${payload.workers.length})`, "ok");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setWorkerControlMessage("Erreur reseau lors du chargement workers.");
      log(`Chargement workers en echec: ${message}`, "error");
    }
  }, [log]);

  // Claims one pending build for selected worker.
  const claimBuildForWorker = useCallback(async () => {
    if (!roleCapabilities.can_run_operations) {
      setWorkerControlMessage("Role insuffisant pour claim build.");
      log(`Role ${adminRole} ne peut pas claim de build`, "warn");
      audit("worker_claim_denied", workerControlForm.worker_id || "unknown");
      return;
    }

    const workerId = workerControlForm.worker_id.trim();
    if (!workerId) {
      setWorkerControlMessage("Worker id requis pour claim.");
      log("Claim worker refuse: worker id manquant", "warn");
      return;
    }

    try {
      const response = await fetch(`/workers/${encodeURIComponent(workerId)}/claim`, {
        method: "POST"
      });

      if (!response.ok) {
        setWorkerControlMessage(`Claim en echec (HTTP ${response.status}).`);
        log(`Claim worker en echec (${workerId}): HTTP ${response.status}`, "error");
        return;
      }

      const payload = (await response.json()) as ClaimBuildResponse;
      if (!payload.build) {
        setLastClaimResult("Aucun build disponible.");
        setWorkerControlMessage("Claim termine: file vide.");
        log(`Claim worker ${workerId}: aucun build disponible`, "info");
        await refreshWorkers();
        return;
      }

      setWorkerControlForm((previous) => ({
        ...previous,
        build_id: payload.build?.id ?? previous.build_id
      }));
      setLastClaimResult(`Build ${payload.build.id.slice(0, 8)} claim.`);
      setWorkerControlMessage(`Claim reussi: build ${payload.build.id.slice(0, 8)}.`);
      log(`Claim worker ${workerId}: build ${payload.build.id.slice(0, 8)}`, "ok");
      audit("worker_claim", workerId);
      await refreshAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setWorkerControlMessage("Erreur reseau lors du claim worker.");
      log(`Claim worker en echec (${workerId}): ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    refreshAll,
    refreshWorkers,
    roleCapabilities.can_run_operations,
    workerControlForm.worker_id
  ]);

  // Completes one claimed build for selected worker.
  const completeBuildForWorker = useCallback(async () => {
    if (!roleCapabilities.can_run_operations) {
      setWorkerControlMessage("Role insuffisant pour complete build.");
      log(`Role ${adminRole} ne peut pas completer de build`, "warn");
      audit("worker_complete_denied", workerControlForm.build_id || "unknown");
      return;
    }

    const workerId = workerControlForm.worker_id.trim();
    const buildId = workerControlForm.build_id.trim();

    if (!workerId || !buildId) {
      setWorkerControlMessage("Worker id et build id requis pour completion.");
      log("Completion worker refusee: worker/build id manquant", "warn");
      return;
    }

    if (workerControlForm.completion_status === "failed") {
      if (!roleCapabilities.can_mutate_sensitive) {
        setWorkerControlMessage("Role insuffisant pour completion failed.");
        log(`Role ${adminRole} ne peut pas forcer un failed`, "warn");
        audit("worker_complete_failed_denied", buildId || "unknown");
        return;
      }

      const confirmed = globalThis.confirm(
        "Confirmer une completion en echec ? Cela peut declencher retry/dead-letter."
      );
      if (!confirmed) {
        setWorkerControlMessage("Completion en echec annulee.");
        return;
      }
    }

    const payload = {
      status: workerControlForm.completion_status,
      log_line: workerControlForm.completion_log_line.trim() || null
    };

    try {
      const response = await fetch(
        `/workers/${encodeURIComponent(workerId)}/builds/${encodeURIComponent(buildId)}/complete`,
        {
          method: "POST",
          headers: {
            "content-type": "application/json"
          },
          body: JSON.stringify(payload)
        }
      );

      if (response.status === 409) {
        setWorkerControlMessage("Conflit de possession: ce build appartient a un autre worker.");
        log(`Completion worker conflit (${workerId}, ${buildId.slice(0, 8)})`, "warn");
        return;
      }

      if (response.status === 400) {
        setWorkerControlMessage("Transition invalide pour completion worker.");
        log(`Completion worker invalide (${workerId}, ${buildId.slice(0, 8)})`, "warn");
        return;
      }

      if (!response.ok) {
        setWorkerControlMessage(`Completion en echec (HTTP ${response.status}).`);
        log(`Completion worker en echec (${workerId}, ${buildId.slice(0, 8)}): HTTP ${response.status}`, "error");
        return;
      }

      const completion = (await response.json()) as CompleteBuildResponse;
      setWorkerControlMessage(
        `Completion reussie: build ${completion.build.id.slice(0, 8)} -> ${completion.build.status}.`
      );
      setWorkerControlForm((previous) => ({ ...previous, build_id: "", completion_log_line: "" }));
      log(
        `Completion worker ${workerId}: build ${completion.build.id.slice(0, 8)} -> ${completion.build.status}`,
        "ok"
      );
      audit("worker_complete", workerId);
      await refreshAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setWorkerControlMessage("Erreur reseau lors de la completion worker.");
      log(`Completion worker en echec (${workerId}, ${buildId.slice(0, 8)}): ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    refreshAll,
    roleCapabilities.can_mutate_sensitive,
    roleCapabilities.can_run_operations,
    workerControlForm
  ]);

  // Reads health endpoint to display backend availability in the HUD.
  const refreshHealth = useCallback(async () => {
    try {
      const response = await fetch("/health", { method: "GET" });
      setHealthStatus(response.ok ? "ok" : "degraded");
    } catch {
      setHealthStatus("degraded");
    }
  }, []);

  // Initializes dashboard data and baseline log once on first mount.
  useEffect(() => {
    log("Console initialisee", "ok");
    void refreshAll();
    void refreshHealth();
  }, [log, refreshAll, refreshHealth]);

  // Polls /health so the top HUD reflects backend availability.
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      void refreshHealth();
    }, 5000);
    return () => globalThis.clearInterval(id);
  }, [refreshHealth]);

  // Keeps stardate indicator updated each minute.
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      setStardate(stardateValue(new Date()));
    }, 60000);
    return () => globalThis.clearInterval(id);
  }, []);

  // Polling fallback ensures updates continue while SSE is disconnected.
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      if (!streamConnected) {
        void refreshAll();
      }
    }, 5000);
    return () => globalThis.clearInterval(id);
  }, [streamConnected, refreshAll]);

  // Opens SSE stream and wires realtime events to logs + snapshot refresh.
  useEffect(() => {
    if (globalThis.EventSource === undefined) {
      log("EventSource non supporte, mode polling uniquement", "warn");
      return;
    }

    const source = new EventSource("/events");

    source.onopen = () => {
      setStreamConnected(true);
      log("Flux temps reel connecte", "ok");
    };

    source.onerror = () => {
      setStreamConnected((previous) => {
        if (previous) {
          log("Perte du flux temps reel, reconnexion en cours", "warn");
        }
        return false;
      });
    };

    source.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data) as LiveEvent;
        pushLiveEvent(payload);
        scheduleRefresh(80);
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        log(`Evenement live invalide: ${message}`, "error");
      }
    };

    return () => {
      source.close();
    };
  }, [log, pushLiveEvent, scheduleRefresh]);

  // Clears any pending debounced refresh timer on unmount.
  useEffect(() => {
    return () => {
      if (refreshTimerRef.current) {
        globalThis.clearTimeout(refreshTimerRef.current);
      }
    };
  }, []);

  // Derived text for stream status chip.
  const streamStatusText = useMemo(
    () => (streamConnected ? "Realtime Online" : "Realtime Offline"),
    [streamConnected]
  );

  // Resolves selected worker details for quick diagnostics in control panel.
  const selectedWorker = useMemo(() => {
    const workerId = workerControlForm.worker_id.trim().toLowerCase();
    if (!workerId) {
      return null;
    }
    return snapshot.workers.find((worker) => worker.id.toLowerCase() === workerId) ?? null;
  }, [snapshot.workers, workerControlForm.worker_id]);

  // Builds one readable allow/deny summary from last plugin authorization dry-run.
  const pluginAuthorizationSummary = useMemo(() => {
    if (!pluginAuthorizationResult) {
      return "Aucun dry-run execute.";
    }

    if (pluginAuthorizationResult.allowed) {
      return `Allow: required=${pluginAuthorizationResult.required_capabilities.join(", ") || "none"}, granted=${pluginAuthorizationResult.granted_capabilities.join(", ") || "none"}.`;
    }

    return `Deny: missing=${pluginAuthorizationResult.missing_capabilities.join(", ") || "none"}.`;
  }, [pluginAuthorizationResult]);

  // Computes per-plugin allow/deny summary against current effective policy context.
  const pluginPolicySummaryByName = useMemo(() => {
    const summary = new Map<string, string>();
    for (const plugin of pluginInventory) {
      const missing = missingCapabilities(plugin.capabilities, effectiveGrantedCapabilities);
      if (missing.length === 0) {
        summary.set(plugin.name, `allow (${effectivePolicyContext})`);
      } else {
        summary.set(plugin.name, `deny (${effectivePolicyContext}) missing: ${missing.join(", ")}`);
      }
    }
    return summary;
  }, [effectiveGrantedCapabilities, effectivePolicyContext, pluginInventory]);

  // Applies advanced filters to live events for observability triage workflows.
  const filteredObservabilityEvents = useMemo(() => {
    const nowMs = Date.now();

    return liveEvents.filter((event) => matchesObservabilityFilter(event, observabilityFilter, nowMs));
  }, [liveEvents, observabilityFilter]);

  // Computes freshness timestamp shown in observability panel.
  const observabilityFreshness = useMemo(() => {
    const latestAt = liveEvents.find((event) => Boolean(event.at))?.at;
    return latestAt ? formatDateTime(latestAt) : formatDateTime(new Date().toISOString());
  }, [liveEvents]);

  // Summarizes build statuses for the Overview page using only dashboard snapshot builds.
  const buildStatusSummary = useMemo(() => {
    const summary = {
      running: 0,
      pending: 0,
      success: 0,
      failed: 0,
      canceled: 0
    };

    for (const build of snapshot.builds) {
      const status = String(build.status).toLowerCase();
      if (status === "running") {
        summary.running += 1;
      } else if (status === "pending") {
        summary.pending += 1;
      } else if (status === "success") {
        summary.success += 1;
      } else if (status === "failed") {
        summary.failed += 1;
      } else if (status === "canceled") {
        summary.canceled += 1;
      }
    }

    return summary;
  }, [snapshot.builds]);

  // Computes one lightweight execution health ratio from build statuses only.
  const deliverySuccessRatio = useMemo(() => {
    const finalBuilds = snapshot.builds.filter((build) => {
      const status = String(build.status).toLowerCase();
      return status === "success" || status === "failed" || status === "canceled";
    });

    if (finalBuilds.length === 0) {
      return "n/a";
    }

    const successCount = finalBuilds.filter((build) => String(build.status).toLowerCase() === "success").length;
    return `${Math.round((successCount / finalBuilds.length) * 100)}%`;
  }, [snapshot.builds]);

  // Builds one read-only execution summary for Workers page using jobs/builds only.
  const workersReadOnlySummary = useMemo(() => {
    const running = snapshot.builds.filter((build) => String(build.status).toLowerCase() === "running").length;
    const pending = snapshot.builds.filter((build) => String(build.status).toLowerCase() === "pending").length;
    const blockedRisk = snapshot.builds.filter((build) => {
      const status = String(build.status).toLowerCase();
      return status === "failed" || status === "canceled";
    }).length;

    return {
      running,
      pending,
      blockedRisk,
      pressure: pending + running > 0 ? `${Math.round((pending / (pending + running)) * 100)}% pending` : "no load"
    };
  }, [snapshot.builds]);

  // Selects a short build sample for execution triage in Workers read-only mode.
  const recentExecutionBuilds = useMemo(() => {
    return snapshot.builds.slice(0, 5);
  }, [snapshot.builds]);

  // Provides one SCM security proxy summary from currently available API-backed data.
  const scmSecurityReadOnlySummary = useMemo(() => {
    const failedBuilds = snapshot.builds.filter((build) => String(build.status).toLowerCase() === "failed").length;
    const canceledBuilds = snapshot.builds.filter((build) => String(build.status).toLowerCase() === "canceled").length;
    const recentJobs = snapshot.jobs.slice(0, 5);

    return {
      failedBuilds,
      canceledBuilds,
      recentJobs,
      apiHealth: healthStatus
    };
  }, [healthStatus, snapshot.builds, snapshot.jobs]);

  // Exports currently filtered observability events as JSON.
  const exportObservabilityJson = useCallback(() => {
    const filename = `observability-events-${new Date().toISOString().replaceAll(":", "-")}.json`;
    downloadTextPayload(filename, JSON.stringify(filteredObservabilityEvents, null, 2), "application/json");
    setObservabilityMessage(`Export JSON genere (${filteredObservabilityEvents.length} events).`);
    log(`Export observability JSON (${filteredObservabilityEvents.length} events)`, "ok");
  }, [filteredObservabilityEvents, log]);

  // Exports currently filtered observability events as CSV.
  const exportObservabilityCsv = useCallback(() => {
    const filename = `observability-events-${new Date().toISOString().replaceAll(":", "-")}.csv`;
    const csv = observabilityEventsToCsv(filteredObservabilityEvents);
    downloadTextPayload(filename, csv, "text/csv;charset=utf-8");
    setObservabilityMessage(`Export CSV genere (${filteredObservabilityEvents.length} events).`);
    log(`Export observability CSV (${filteredObservabilityEvents.length} events)`, "ok");
  }, [filteredObservabilityEvents, log]);

  // Keeps roadmap-only state/actions referenced while pages are progressively wired.
  keepRoadmapReferences(
    severityToStatusClass,
    formatTime,
    PLUGIN_CAPABILITY_OPTIONS,
    adminActivity,
    setAdminActivity,
    webhookForm,
    setWebhookForm,
    webhookMessage,
    setWebhookMessage,
    showWebhookSecret,
    setShowWebhookSecret,
    knownWebhookConfigs,
    setKnownWebhookConfigs,
    pollingForm,
    setPollingForm,
    pollingMessage,
    setPollingMessage,
    pollingTickSummary,
    setPollingTickSummary,
    knownPollingStates,
    setKnownPollingStates,
    workerControlForm,
    setWorkerControlForm,
    workerControlMessage,
    setWorkerControlMessage,
    lastClaimResult,
    setLastClaimResult,
    pluginAdminForm,
    setPluginAdminForm,
    pluginAdminMessage,
    setPluginAdminMessage,
    pluginInventory,
    setPluginInventory,
    pluginPolicyForm,
    setPluginPolicyForm,
    pluginPolicyMessage,
    setPluginPolicyMessage,
    pluginAuthorizationResult,
    setPluginAuthorizationResult,
    effectivePolicyContext,
    setEffectivePolicyContext,
    effectiveGrantedCapabilities,
    setEffectiveGrantedCapabilities,
    scmWebhookOpsFilter,
    setScmWebhookOpsFilter,
    scmWebhookOpsMessage,
    setScmWebhookOpsMessage,
    scmWebhookMetrics,
    setScmWebhookMetrics,
    scmWebhookRejections,
    setScmWebhookRejections,
    saveWebhookSecurityConfig,
    saveScmPollingConfig,
    runManualScmPollingTick,
    refreshPluginInventory,
    loadPlugin,
    initPlugin,
    executePlugin,
    unloadPlugin,
    togglePluginPolicyCapability,
    loadPluginPolicy,
    savePluginPolicy,
    runPluginAuthorizationCheck,
    refreshScmWebhookOperations,
    refreshWorkers,
    claimBuildForWorker,
    completeBuildForWorker,
    observabilityFilter,
    setObservabilityFilter,
    observabilityMessage,
    setObservabilityMessage,
    observabilityFreshness,
    exportObservabilityJson,
    exportObservabilityCsv,
    selectedWorker,
    pluginAuthorizationSummary,
    pluginPolicySummaryByName
  );

  return (
    <>
      <div className="bg-orb orb-1"></div>
      <div className="bg-orb orb-2"></div>
      <div className="bg-orb orb-3"></div>
      <div className="bg-grid"></div>
      <div className="bg-scanline"></div>

      <main className="shell">
        <section className="hud-strip reveal" style={{ ["--delay" as string]: "0s" }}>
          <span>Deck: CI-01</span>
          <span>Channel: Build Control</span>
          <span>Stardate: {stardate}</span>
        </section>

        <header className="hero">
          <div className="hero-copy-wrap">
            <div className="logo-shell" aria-hidden="true">
              <img className="tardi-logo" src="/tardigrade-logo.png" alt="" />
            </div>
            <div className="hero-copy">
              <p className="eyebrow">Bridge Control Plane</p>
              <h1>Tardigrade CI Console</h1>
              <p className="subtitle">
                Console multi-pages alignee sur les fonctions API disponibles, avec extension progressive vers la cible UX.
              </p>
            </div>
          </div>
          <div className="hero-actions">
            <div className={`status-chip ${streamConnected ? "connected" : "disconnected"}`}>
              {streamStatusText}
            </div>
            <div className={`status-chip ${healthStatus === "ok" ? "connected" : "disconnected"}`}>
              API {healthStatus === "ok" ? "Healthy" : "Degraded"}
            </div>
            <label>
              <span>Role</span>
              <select
                name="admin_role"
                value={adminRole}
                onChange={(event) => setAdminRole(event.target.value as AdminRole)}
              >
                <option value="viewer">viewer</option>
                <option value="operator">operator</option>
                <option value="admin">admin</option>
              </select>
            </label>
            <button className="btn btn-ghost" onClick={() => void refreshAll()} type="button">
              Synchroniser
            </button>
          </div>
        </header>

        <section className="page-nav reveal" style={{ ["--delay" as string]: "0.01s" }}>
          {DASHBOARD_NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`page-tab ${activePage === item.id ? "active" : ""}`}
              onClick={() => setActivePage(item.id)}
            >
              {item.label}
            </button>
          ))}
        </section>

        <section className="panel reveal api-coverage-panel" style={{ ["--delay" as string]: "0.015s" }}>
          <div className="panel-head">
            <h2>Perimetre API reel</h2>
            <span className="pill">{activeCoverage}</span>
          </div>
          <p className="hint">
            Endpoints disponibles: GET /health, POST /jobs, GET /jobs, POST /jobs/{"{id}"}/run,
            POST /builds/{"{id}"}/cancel, GET /builds.
          </p>
        </section>

        <section className="grid">
          {isImplementedPage ? (
            <>
              {activePage === "pipelines" && (
                <article className="panel panel-form reveal" style={{ ["--delay" as string]: "0.02s" }}>
                  <h2>Nouveau Job</h2>
                  <form className="form" onSubmit={(event) => void createJob(event)}>
                    <label>
                      <span>Nom du job</span>
                      <input
                        name="name"
                        placeholder="build-api"
                        required
                        value={form.name}
                        onChange={(event) => setForm((prev) => ({ ...prev, name: event.target.value }))}
                      />
                    </label>
                    <label>
                      <span>Depot git</span>
                      <input
                        name="repository_url"
                        placeholder="https://example.com/project.git"
                        required
                        value={form.repository_url}
                        onChange={(event) => setForm((prev) => ({ ...prev, repository_url: event.target.value }))}
                      />
                    </label>
                    <label>
                      <span>Pipeline file</span>
                      <input
                        name="pipeline_path"
                        placeholder="pipelines/api.yml"
                        required
                        value={form.pipeline_path}
                        onChange={(event) => setForm((prev) => ({ ...prev, pipeline_path: event.target.value }))}
                      />
                    </label>
                    <button type="submit" className="btn btn-primary">
                      POST /jobs
                    </button>
                  </form>
                  <p className="hint">{createMessage}</p>
                </article>
              )}

              {(activePage === "pipelines" || activePage === "overview") && (
                <article className="panel reveal" style={{ ["--delay" as string]: "0.12s" }}>
            <div className="panel-head">
              <h2>Jobs</h2>
              <span className="pill">{snapshot.jobs.length}</span>
            </div>
            <div className="list">
              {snapshot.jobs.length === 0 ? (
                <p className="hint">Aucun job pour le moment.</p>
              ) : (
                snapshot.jobs.map((job) => (
                  <div className="list-item job-item" key={job.id}>
                    <div>
                      <p className="item-title">{job.name}</p>
                      <p className="item-subtitle">
                        {job.repository_url} | {job.pipeline_path}
                      </p>
                    </div>
                    <div className="actions">
                      <button className="btn btn-small btn-secondary" type="button" onClick={() => void runJob(job.id, job.name)}>
                        POST /jobs/{"{id}"}/run
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
                </article>
              )}

              {(activePage === "pipelines" || activePage === "overview") && (
                <article className="panel reveal" style={{ ["--delay" as string]: "0.22s" }}>
            <div className="panel-head">
              <h2>Builds</h2>
              <span className="pill">{snapshot.builds.length}</span>
            </div>
            <div className="list">
              {snapshot.builds.length === 0 ? (
                <p className="hint">Aucun build encore lance.</p>
              ) : (
                snapshot.builds.map((build) => {
                  const isFinal =
                    build.status === "Canceled" || build.status === "Success" || build.status === "Failed";

                  return (
                    <div className="list-item build-item" key={build.id}>
                      <div>
                        <p className="item-title">Build {build.id.slice(0, 8)}</p>
                        <p className="item-subtitle">
                          Job {build.job_id.slice(0, 8)} | {formatDateTime(build.queued_at)}
                        </p>
                      </div>
                      <div className="actions">
                        <span className={`status ${String(build.status).toLowerCase()}`}>{build.status}</span>
                        <button
                          className="btn btn-small btn-warning"
                          type="button"
                          disabled={isFinal}
                          onClick={() => void cancelBuild(build.id)}
                          style={isFinal ? { opacity: 0.4, cursor: "default" } : undefined}
                        >
                          POST /builds/{"{id}"}/cancel
                        </button>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
                </article>
              )}

              {activePage === "overview" && (
                <article className="panel panel-metrics reveal" style={{ ["--delay" as string]: "0.3s" }}>
            <div className="panel-head">
              <h2>Health & Delivery Snapshot</h2>
              <span className="pill">live</span>
            </div>
            <div className="metrics-grid">
              <div className="metric-card">
                <p className="metric-label">API Health</p>
                <p className="metric-value">{healthStatus === "ok" ? "OK" : "DEGRADED"}</p>
              </div>
              <div className="metric-card">
                <p className="metric-label">Jobs</p>
                <p className="metric-value">{snapshot.jobs.length}</p>
              </div>
              <div className="metric-card">
                <p className="metric-label">Builds</p>
                <p className="metric-value">{snapshot.builds.length}</p>
              </div>
              <div className="metric-card">
                <p className="metric-label">Success Ratio</p>
                <p className="metric-value">{deliverySuccessRatio}</p>
              </div>
            </div>
                </article>
              )}

              {activePage === "overview" && (
                <article className="panel reveal" style={{ ["--delay" as string]: "0.31s" }}>
            <div className="panel-head">
              <h2>Build Status Breakdown</h2>
              <span className="pill">{snapshot.builds.length}</span>
            </div>
            <div className="list">
              <div className="list-item">
                <div>
                  <p className="item-title">Running</p>
                  <p className="item-subtitle">In-progress builds from GET /builds</p>
                </div>
                <div className="actions">
                  <span className="status pending">{buildStatusSummary.running}</span>
                </div>
              </div>
              <div className="list-item">
                <div>
                  <p className="item-title">Pending</p>
                  <p className="item-subtitle">Queued builds awaiting execution</p>
                </div>
                <div className="actions">
                  <span className="status pending">{buildStatusSummary.pending}</span>
                </div>
              </div>
              <div className="list-item">
                <div>
                  <p className="item-title">Success</p>
                  <p className="item-subtitle">Completed successful executions</p>
                </div>
                <div className="actions">
                  <span className="status success">{buildStatusSummary.success}</span>
                </div>
              </div>
              <div className="list-item">
                <div>
                  <p className="item-title">Failed / Canceled</p>
                  <p className="item-subtitle">Final non-success states</p>
                </div>
                <div className="actions">
                  <span className="status failed">{buildStatusSummary.failed + buildStatusSummary.canceled}</span>
                </div>
              </div>
            </div>
                </article>
              )}

              {activePage === "overview" && (
                <article className="panel reveal" style={{ ["--delay" as string]: "0.315s" }}>
            <div className="panel-head">
              <h2>API-backed Overview Scope</h2>
              <span className="pill">strict</span>
            </div>
            <div className="list">
              <div className="list-item">
                <div>
                  <p className="item-title">Data sources in use</p>
                  <p className="item-subtitle">GET /health, GET /jobs, GET /builds</p>
                </div>
              </div>
              <div className="list-item">
                <div>
                  <p className="item-title">Roadmap-only metrics excluded</p>
                  <p className="item-subtitle">No reliance on /metrics, /events, /dead-letter-builds for this page.</p>
                </div>
              </div>
              <div className="list-item">
                <div>
                  <p className="item-title">Freshness</p>
                  <p className="item-subtitle">Last UI refresh: {formatDateTime(new Date().toISOString())}</p>
                </div>
              </div>
            </div>
                </article>
              )}
            </>
          ) : (
            <>
              {activePage === "workers" ? (
                <>
                  <article className="panel reveal" style={{ ["--delay" as string]: "0.02s" }}>
                    <h2>Page en mode roadmap</h2>
                    <p className="hint">
                      Vue Workers partiellement activee en read-only a partir de GET /jobs et GET /builds.
                    </p>
                    <div className="list">
                      <div className="list-item">
                        <div>
                          <p className="item-title">API coverage: roadmap</p>
                          <p className="item-subtitle">
                            Controle worker (claim/complete/fleet health) en attente d'endpoints publics dedies.
                          </p>
                        </div>
                      </div>
                    </div>
                  </article>

                  <article className="panel panel-metrics reveal" style={{ ["--delay" as string]: "0.06s" }}>
                    <div className="panel-head">
                      <h2>Execution Pressure (read-only)</h2>
                      <span className="pill">derived</span>
                    </div>
                    <div className="metrics-grid">
                      <div className="metric-card">
                        <p className="metric-label">Running builds</p>
                        <p className="metric-value">{workersReadOnlySummary.running}</p>
                      </div>
                      <div className="metric-card">
                        <p className="metric-label">Pending builds</p>
                        <p className="metric-value">{workersReadOnlySummary.pending}</p>
                      </div>
                      <div className="metric-card">
                        <p className="metric-label">Failure risk</p>
                        <p className="metric-value">{workersReadOnlySummary.blockedRisk}</p>
                      </div>
                      <div className="metric-card">
                        <p className="metric-label">Queue pressure</p>
                        <p className="metric-value">{workersReadOnlySummary.pressure}</p>
                      </div>
                    </div>
                  </article>

                  <article className="panel reveal" style={{ ["--delay" as string]: "0.1s" }}>
                    <div className="panel-head">
                      <h2>Recent Execution Sample</h2>
                      <span className="pill">{recentExecutionBuilds.length}</span>
                    </div>
                    <div className="list">
                      {recentExecutionBuilds.length === 0 ? (
                        <p className="hint">Aucun build disponible pour le moment.</p>
                      ) : (
                        recentExecutionBuilds.map((build) => (
                          <div className="list-item" key={`workers-sample-${build.id}`}>
                            <div>
                              <p className="item-title">Build {build.id.slice(0, 8)}</p>
                              <p className="item-subtitle">Job {build.job_id.slice(0, 8)} | {formatDateTime(build.queued_at)}</p>
                            </div>
                            <div className="actions">
                              <span className={`status ${String(build.status).toLowerCase()}`}>{build.status}</span>
                            </div>
                          </div>
                        ))
                      )}
                    </div>
                  </article>
                </>
              ) : (
                <>
                  {activePage === "scm-security" && (
                    <>
                      <article className="panel reveal" style={{ ["--delay" as string]: "0.02s" }}>
                        <h2>Page en mode roadmap</h2>
                        <p className="hint">
                          Vue SCM Security partiellement activee en read-only a partir de GET /health, GET /jobs, GET /builds.
                        </p>
                        <div className="list">
                          <div className="list-item">
                            <div>
                              <p className="item-title">API coverage: roadmap</p>
                              <p className="item-subtitle">
                                Configuration webhook/polling et diagnostics SCM en attente d'endpoints publics dedies.
                              </p>
                            </div>
                          </div>
                        </div>
                      </article>

                      <article className="panel panel-metrics reveal" style={{ ["--delay" as string]: "0.06s" }}>
                        <div className="panel-head">
                          <h2>SCM Risk Proxy (read-only)</h2>
                          <span className="pill">derived</span>
                        </div>
                        <div className="metrics-grid">
                          <div className="metric-card">
                            <p className="metric-label">API Health</p>
                            <p className="metric-value">{scmSecurityReadOnlySummary.apiHealth === "ok" ? "OK" : "DEGRADED"}</p>
                          </div>
                          <div className="metric-card">
                            <p className="metric-label">Failed builds</p>
                            <p className="metric-value">{scmSecurityReadOnlySummary.failedBuilds}</p>
                          </div>
                          <div className="metric-card">
                            <p className="metric-label">Canceled builds</p>
                            <p className="metric-value">{scmSecurityReadOnlySummary.canceledBuilds}</p>
                          </div>
                          <div className="metric-card">
                            <p className="metric-label">Recent jobs</p>
                            <p className="metric-value">{scmSecurityReadOnlySummary.recentJobs.length}</p>
                          </div>
                        </div>
                      </article>

                      <article className="panel reveal" style={{ ["--delay" as string]: "0.1s" }}>
                        <div className="panel-head">
                          <h2>Recent SCM Sources (from jobs)</h2>
                          <span className="pill">{scmSecurityReadOnlySummary.recentJobs.length}</span>
                        </div>
                        <div className="list">
                          {scmSecurityReadOnlySummary.recentJobs.length === 0 ? (
                            <p className="hint">Aucun job disponible pour le moment.</p>
                          ) : (
                            scmSecurityReadOnlySummary.recentJobs.map((job) => (
                              <div className="list-item" key={`scm-source-${job.id}`}>
                                <div>
                                  <p className="item-title">{job.name}</p>
                                  <p className="item-subtitle">{job.repository_url}</p>
                                  <p className="item-subtitle">Pipeline: {job.pipeline_path}</p>
                                </div>
                              </div>
                            ))
                          )}
                        </div>
                      </article>
                    </>
                  )}

                  {activePage !== "scm-security" && (
                <article className="panel reveal" style={{ ["--delay" as string]: "0.02s" }}>
                  <h2>Page en mode roadmap</h2>
                  <p className="hint">
                    Cette page correspond a la maquette cible mais n'est pas encore reliee aux endpoints exposes.
                  </p>
                  <div className="list">
                    <div className="list-item">
                      <div>
                        <p className="item-title">API coverage: roadmap</p>
                        <p className="item-subtitle">Prochaine etape: ajouter les endpoints backend puis brancher les actions UI.</p>
                      </div>
                    </div>
                  </div>
                </article>
                  )}
                </>
              )}
            </>
          )}
        </section>

        <section className="panel console reveal" style={{ ["--delay" as string]: "0.32s" }}>
          <h2>Journal de bord</h2>
          <pre aria-live="polite">{logs}</pre>
        </section>
      </main>
    </>
  );
}
