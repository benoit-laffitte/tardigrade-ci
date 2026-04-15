import type { AdministrationPageProps } from "./types";

// Renders the Administration roadmap page with governance/autonomy signals from controller state.
export function AdministrationPage({ adminActivity, roleCapabilities }: Readonly<AdministrationPageProps>) {
  return (
    <>
      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.02s" }}>
        <h2>Triage administration</h2>
        <div className="metrics-grid">
          <div className="metric-card"><p className="metric-label">Pending approvals</p><p className="metric-value">6</p></div>
          <div className="metric-card"><p className="metric-label">Privilege drift</p><p className="metric-value">{roleCapabilities.can_mutate_sensitive ? 0 : 3}</p></div>
          <div className="metric-card"><p className="metric-label">Sensitive ops</p><p className="metric-value">{adminActivity.length}</p></div>
          <div className="metric-card"><p className="metric-label">Coverage</p><p className="metric-value">roadmap</p></div>
        </div>
      </article>

      <article className="panel panel-third reveal" style={{ ["--delay" as string]: "0.06s" }}>
        <h2>Runbook gouvernance</h2>
        <p className="hint">Actions de containment admin (roadmap).</p>
      </article>

      <article className="panel panel-two-thirds reveal" style={{ ["--delay" as string]: "0.1s" }}>
        <h2>Role & capability coverage</h2>
        <p className="hint">Vue RBAC structurante conforme maquette.</p>
      </article>

      <article className="panel panel-half reveal" style={{ ["--delay" as string]: "0.14s" }}>
        <h2>Sensitive operations control</h2>
      </article>

      <article className="panel panel-half reveal" style={{ ["--delay" as string]: "0.18s" }}>
        <h2>Admin access anomalies</h2>
      </article>

      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.22s" }}>
        <h2>Change approvals & maintenance windows</h2>
      </article>

      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.26s" }}>
        <h2>Admin activity</h2>
        <div className="list">
          {adminActivity.length === 0 ? (
            <p className="hint">Aucune activite admin recente.</p>
          ) : (
            adminActivity.slice(0, 12).map((entry, index) => (
              <div className="list-item" key={`admin-activity-${index}-${entry.at}`}>
                <div>
                  <p className="item-title">{entry.action}</p>
                  <p className="item-subtitle">Target: {entry.target}</p>
                  <p className="item-subtitle">Role: {entry.actor_role} | {new Date(entry.at).toLocaleString()}</p>
                </div>
              </div>
            ))
          )}
        </div>
      </article>
    </>
  );
}