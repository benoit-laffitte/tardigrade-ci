import type { ScmSecurityPageProps } from "./types";
import { useScmDomain } from "../hooks/scm/domain";
import { useWebhookSecurityConfig } from "../hooks/scm/useWebhookSecurityConfig";

// Renders the SCM Security page in roadmap/read-only mode.
export function ScmSecurityPage({
  scmSecurityReadOnlySummary,
  adminRole,
  roleCapabilities,
  log,
  audit,
  refreshAll
}: Readonly<ScmSecurityPageProps>) {
  const scmDomain = useScmDomain({
    adminRole,
    roleCapabilities,
    log,
    audit,
    refreshAll
  });

  return (
    <>
      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.02s" }}>
        <h2>Page en mode roadmap</h2>
        <p className="hint">
          Vue SCM Security en transition: diagnostics webhook et configuration webhook passent par GraphQL.
        </p>
        <div className="list">
          <div className="list-item">
            <div>
              <p className="item-title">API coverage: roadmap</p>
              <p className="item-subtitle">
                Webhooks natifs preserves via un adaptateur HTTP dedie. Le polling SCM est pilote via GraphQL.
              </p>
            </div>
          </div>
        </div>
      </article>

      <article className="panel panel-half panel-metrics reveal" style={{ ["--delay" as string]: "0.06s" }}>
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

      <article className="panel panel-half reveal" style={{ ["--delay" as string]: "0.1s" }}>
        <div className="panel-head">
          <h2>Recent SCM Sources (from jobs)</h2>
          <span className="pill">{scmSecurityReadOnlySummary.recentJobs.length}</span>
        </div>
        <div className="list">
          {scmSecurityReadOnlySummary.recentJobs.length === 0 ? (
            <p className="hint">Aucun job disponible pour le moment.</p>
          ) : (
            scmSecurityReadOnlySummary.recentJobs.map((job) => {
              // On suppose provider = "Github" pour démo, à adapter si info disponible côté job
              const { config, loading, error } = useWebhookSecurityConfig(job.repository_url, "Github");
              return (
                <div className="list-item" key={`scm-source-${job.id}`}>
                  <div>
                    <p className="item-title">{job.name}</p>
                    <p className="item-subtitle">{job.repository_url}</p>
                    <p className="item-subtitle">Pipeline: {job.pipeline_path}</p>
                    <div className="item-subtitle">
                      {loading && <span>Chargement config webhook…</span>}
                      {error && <span style={{ color: "red" }}>Erreur: {String(error)}</span>}
                      {config && (
                        <>
                          <span>Webhook: <b>{config.repository_url}</b> [{config.provider}]</span><br />
                          <span>Secret: <b>{config.secret_masked || "(non défini)"}</b></span><br />
                          <span>IPs autorisées: {config.allowed_ips.length > 0 ? config.allowed_ips.join(", ") : "(aucune)"}</span>
                        </>
                      )}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </article>

      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.14s" }}>
        <div className="panel-head">
          <h2>SCM Domain Signals</h2>
          <span className="pill">domain</span>
        </div>
        <p className="hint">{scmDomain.webhookMessage || "Aucune operation webhook recente."}</p>
        <p className="hint">Adaptateur natif attendu sur /webhooks/scm pour les fournisseurs SCM.</p>
        <p className="hint">{scmDomain.pollingMessage || "Aucune operation polling recente."}</p>
        <p className="hint">{scmDomain.scmWebhookOpsMessage || "Aucun refresh diagnostics recent."}</p>
        {scmDomain.pollingTickSummary ? (
          <p className="hint">
            Tick: repos={scmDomain.pollingTickSummary.polled_repositories}, enqueued={scmDomain.pollingTickSummary.enqueued_builds}
          </p>
        ) : null}
        <p className="hint">Rejections loaded: {scmDomain.scmWebhookRejections.length}</p>
        <div className="actions">
          <button className="btn ghost" onClick={() => void scmDomain.runManualScmPollingTick()}>
            Run polling tick
          </button>
          <button className="btn ghost" onClick={() => void scmDomain.refreshScmWebhookOperations()}>
            Refresh webhook diagnostics
          </button>
        </div>
      </article>
    </>
  );
}