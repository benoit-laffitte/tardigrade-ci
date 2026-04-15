import { useMemo } from "react";

import { ADMIN_ROLE_CAPABILITIES, DASHBOARD_NAV_ITEMS, PAGE_PRESENTATION } from "../dashboardConstants";
import type {
  AdminRole,
  DashboardPage,
  DashboardSnapshot,
  HealthStatus,
  LiveEvent,
  ObservabilityFilter,
  PluginAuthorizationCheckResponse,
  PluginInfo,
  Worker,
  WorkerControlInput
} from "../dashboardTypes";
import { formatDateTime, matchesObservabilityFilter, missingCapabilities } from "../dashboardUtils";

interface DerivedStateParams {
  activePage: DashboardPage;
  adminRole: AdminRole;
  snapshot: DashboardSnapshot;
  streamConnected: boolean;
  workerControlForm: WorkerControlInput;
  pluginAuthorizationResult: PluginAuthorizationCheckResponse | null;
  pluginInventory: PluginInfo[];
  effectiveGrantedCapabilities: string[];
  effectivePolicyContext: string;
  liveEvents: LiveEvent[];
  observabilityFilter: ObservabilityFilter;
  healthStatus: HealthStatus;
}

// Computes all memoized dashboard view-model values derived from controller state.
export function useDerivedState({
  activePage,
  adminRole,
  snapshot,
  streamConnected,
  workerControlForm,
  pluginAuthorizationResult,
  pluginInventory,
  effectiveGrantedCapabilities,
  effectivePolicyContext,
  liveEvents,
  observabilityFilter,
  healthStatus
}: Readonly<DerivedStateParams>) {
  // Resolves API coverage label for currently selected page in the navigation bar.
  const activeCoverage = useMemo(() => {
    const current = DASHBOARD_NAV_ITEMS.find((item) => item.id === activePage);
    return current?.coverage ?? "roadmap";
  }, [activePage]);

  // Identifies pages already wired to the currently available API surface.
  const isImplementedPage = activePage === "pipelines" || activePage === "overview";

  // Checks whether current role grants operation/sensitive mutation capability.
  const roleCapabilities = useMemo(() => ADMIN_ROLE_CAPABILITIES[adminRole], [adminRole]);

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
    return snapshot.workers.find((worker: Worker) => worker.id.toLowerCase() === workerId) ?? null;
  }, [snapshot.workers, workerControlForm.worker_id]);

  // Builds one readable allow/deny summary from the last plugin authorization dry-run.
  const pluginAuthorizationSummary = useMemo(() => {
    if (!pluginAuthorizationResult) {
      return "Aucun dry-run execute.";
    }

    if (pluginAuthorizationResult.allowed) {
      return `Allow: required=${pluginAuthorizationResult.required_capabilities.join(", ") || "none"}, granted=${pluginAuthorizationResult.granted_capabilities.join(", ") || "none"}.`;
    }

    return `Deny: missing=${pluginAuthorizationResult.missing_capabilities.join(", ") || "none"}.`;
  }, [pluginAuthorizationResult]);

  // Computes per-plugin allow/deny summary against the current effective policy context.
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

  // Computes freshness timestamp shown in the observability panel.
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

  // Builds one read-only execution summary for the Workers page using jobs/builds only.
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
  const recentExecutionBuilds = useMemo(() => snapshot.builds.slice(0, 5), [snapshot.builds]);

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

  // Resolves the current page heading metadata shown in the topbar.
  const currentPage = useMemo(() => PAGE_PRESENTATION[activePage], [activePage]);

  return {
    activeCoverage,
    isImplementedPage,
    roleCapabilities,
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
  };
}