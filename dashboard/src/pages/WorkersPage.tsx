import type { WorkersPageProps } from "./types";
import { useWorkerDomain } from "../hooks/workers/domain";

// Renders the Workers page in roadmap/read-only mode.
export function WorkersPage({
  workersReadOnlySummary,
  recentExecutionBuilds,
  adminRole,
  roleCapabilities,
  setWorkersSnapshot,
  log,
  audit,
  refreshAll,
  formatDateTime
}: Readonly<WorkersPageProps>) {
  const workerDomain = useWorkerDomain({
    adminRole,
    roleCapabilities,
    setWorkersSnapshot,
    log,
    audit,
    refreshAll
  });

  return (
    <>
      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.02s" }}>
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

      <article className="panel panel-third panel-metrics reveal" style={{ ["--delay" as string]: "0.06s" }}>
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
        <p className="hint">{workerDomain.workerControlMessage || "Worker control non declenche."}</p>
        {workerDomain.lastClaimResult ? <p className="hint">Last claim: {workerDomain.lastClaimResult}</p> : null}
        <div className="actions">
          <button className="btn ghost" onClick={() => void workerDomain.refreshWorkers()}>
            Refresh workers domain
          </button>
        </div>
      </article>

      <article className="panel panel-two-thirds reveal" style={{ ["--delay" as string]: "0.1s" }}>
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
  );
}