import type { OverviewPageProps } from "./types";

// Renders the health-oriented Overview page from API-backed delivery data.
export function OverviewPage({
  jobs,
  builds,
  healthStatus,
  deliverySuccessRatio,
  buildStatusSummary,
  formatDateTime
}: Readonly<OverviewPageProps>) {
  return (
    <>
      <article className="panel panel-two-thirds reveal" style={{ ["--delay" as string]: "0.12s" }}>
        <div className="panel-head">
          <h2>Jobs</h2>
          <span className="pill">{jobs.length}</span>
        </div>
        <div className="list">
          {jobs.length === 0 ? (
            <p className="hint">Aucun job pour le moment.</p>
          ) : (
            jobs.map((job) => (
              <div className="list-item job-item" key={job.id}>
                <div>
                  <p className="item-title">{job.name}</p>
                  <p className="item-subtitle">
                    {job.repository_url} | {job.pipeline_path}
                  </p>
                </div>
              </div>
            ))
          )}
        </div>
      </article>

      <article className="panel panel-third reveal" style={{ ["--delay" as string]: "0.22s" }}>
        <div className="panel-head">
          <h2>Builds</h2>
          <span className="pill">{builds.length}</span>
        </div>
        <div className="list">
          {builds.length === 0 ? (
            <p className="hint">Aucun build encore lance.</p>
          ) : (
            builds.map((build) => (
              <div className="list-item build-item" key={build.id}>
                <div>
                  <p className="item-title">Build {build.id.slice(0, 8)}</p>
                  <p className="item-subtitle">
                    Job {build.job_id.slice(0, 8)} | {formatDateTime(build.queued_at)}
                  </p>
                </div>
                <div className="actions">
                  <span className={`status ${String(build.status).toLowerCase()}`}>{build.status}</span>
                </div>
              </div>
            ))
          )}
        </div>
      </article>

      <article className="panel panel-half panel-metrics reveal" style={{ ["--delay" as string]: "0.3s" }}>
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
            <p className="metric-value">{jobs.length}</p>
          </div>
          <div className="metric-card">
            <p className="metric-label">Builds</p>
            <p className="metric-value">{builds.length}</p>
          </div>
          <div className="metric-card">
            <p className="metric-label">Success Ratio</p>
            <p className="metric-value">{deliverySuccessRatio}</p>
          </div>
        </div>
      </article>

      <article className="panel panel-third reveal" style={{ ["--delay" as string]: "0.31s" }}>
        <div className="panel-head">
          <h2>Build Status Breakdown</h2>
          <span className="pill">{builds.length}</span>
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

      <article className="panel panel-third reveal" style={{ ["--delay" as string]: "0.315s" }}>
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
    </>
  );
}