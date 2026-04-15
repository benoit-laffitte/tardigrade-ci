import { useApolloClient } from "@apollo/client";
import { useCallback, useMemo, useRef, useState } from "react";

import {
  CANCEL_BUILD_MUTATION,
  CREATE_JOB_MUTATION,
  ADMIN_ROLE_CAPABILITIES,
  DASHBOARD_NAV_ITEMS,
  DASHBOARD_SNAPSHOT_QUERY,
  RUN_JOB_MUTATION
} from "../dashboardConstants";
import type {
  AdminActivityEntry,
  AdminRole,
  CancelBuildResponse,
  CreateJobInput,
  CreateJobResponse,
  DashboardPage,
  DashboardSnapshot,
  DashboardSnapshotResponse,
  HealthStatus,
  LiveEvent,
  ObservabilityFilter,
  PluginAuthorizationCheckResponse,
  PluginInfo,
  RunJobResponse,
  WorkerControlInput
} from "../dashboardTypes";
import {
  downloadTextPayload,
  formatDateTime,
  observabilityEventsToCsv,
  stardateValue
} from "../dashboardUtils";
import { useDerivedState } from "./derivedState";
import { useRuntimeEffects } from "./runtimeEffects";

// Encapsulates dashboard state, side-effects, API handlers, and derived view models.
// eslint-disable-next-line sonarjs/cognitive-complexity
export function useController() {
  const client = useApolloClient();
  const refreshTimerRef = useRef<number | null>(null);

  const [streamConnected, setStreamConnected] = useState(false);
  const [activePage, setActivePage] = useState<DashboardPage>("pipelines");
  const [healthStatus, setHealthStatus] = useState<HealthStatus>("degraded");
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
  const [observabilityFilter] = useState<ObservabilityFilter>({
    severity: "",
    kind: "",
    resource_id: "",
    window_minutes: "15"
  });
  const [observabilityMessage, setObservabilityMessage] = useState("");
  const [stardate, setStardate] = useState(() => stardateValue(new Date()));

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

  // Roadmap domains are page-owned; controller provides neutral defaults for global derivations.
  const neutralWorkerControlForm = useMemo<WorkerControlInput>(
    () => ({ worker_id: "", build_id: "", completion_status: "success", completion_log_line: "" }),
    []
  );
  const neutralPluginAuthorization: PluginAuthorizationCheckResponse | null = null;
  const neutralPluginInventory: PluginInfo[] = [];
  const neutralGrantedCapabilities: string[] = [];
  const neutralPolicyContext = "global";

  const {
    activeCoverage,
    isImplementedPage,
    streamStatusText,
    selectedWorker,
    pluginAuthorizationSummary,
    pluginPolicySummaryByName,
    filteredObservabilityEvents,
    observabilityFreshness,
    buildStatusSummary,
    deliverySuccessRatio,
    workersReadOnlySummary,
    recentExecutionBuilds,
    scmSecurityReadOnlySummary,
    currentPage
  } = useDerivedState({
    activePage,
    adminRole,
    snapshot,
    streamConnected,
    workerControlForm: neutralWorkerControlForm,
    pluginAuthorizationResult: neutralPluginAuthorization,
    pluginInventory: neutralPluginInventory,
    effectiveGrantedCapabilities: neutralGrantedCapabilities,
    effectivePolicyContext: neutralPolicyContext,
    liveEvents,
    observabilityFilter,
    healthStatus
  });

  useRuntimeEffects({
    log,
    refreshAll,
    refreshTimerRef,
    streamConnected,
    setStreamConnected,
    setHealthStatus,
    setStardate,
    pushLiveEvent,
    scheduleRefresh
  });

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

  return {
    streamConnected,
    activePage,
    setActivePage,
    healthStatus,
    adminRole,
    setAdminRole,
    logs,
    createMessage,
    form,
    setForm,
    snapshot,
    stardate,
    activeCoverage,
    isImplementedPage,
    streamStatusText,
    currentPage,
    workersReadOnlySummary,
    recentExecutionBuilds,
    scmSecurityReadOnlySummary,
    adminActivity,
    roleCapabilities,
    deliverySuccessRatio,
    buildStatusSummary,
    setSnapshot,
    log,
    audit,
    observabilityMessage,
    setObservabilityMessage,
    observabilityFreshness,
    exportObservabilityJson,
    exportObservabilityCsv,
    selectedWorker,
    pluginAuthorizationSummary,
    pluginPolicySummaryByName,
    refreshAll,
    runJob,
    cancelBuild,
    createJob,
    formatDateTime,
    DASHBOARD_NAV_ITEMS
  };
}
