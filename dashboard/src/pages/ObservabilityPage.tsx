// Renders the Observability page placeholder aligned with the validated mockup.
export function ObservabilityPage() {
  return (
    <>
      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.02s" }}>
        <h2>Triage observabilite</h2>
        <div className="metrics-grid">
          <div className="metric-card"><p className="metric-label">Critical alerts</p><p className="metric-value">4</p></div>
          <div className="metric-card"><p className="metric-label">Event burst</p><p className="metric-value">+38%</p></div>
          <div className="metric-card"><p className="metric-label">Signal lag</p><p className="metric-value">22s</p></div>
          <div className="metric-card"><p className="metric-label">Coverage</p><p className="metric-value">roadmap</p></div>
        </div>
      </article>

      <article className="panel panel-third reveal" style={{ ["--delay" as string]: "0.06s" }}>
        <h2>Actions guidees</h2>
        <p className="hint">Runbook investigation (roadmap).</p>
      </article>

      <article className="panel panel-two-thirds reveal" style={{ ["--delay" as string]: "0.1s" }}>
        <h2>Live event stream</h2>
        <p className="hint">Disposition two-thirds conservee pour le flux principal.</p>
      </article>

      <article className="panel panel-half reveal" style={{ ["--delay" as string]: "0.14s" }}>
        <h2>Signal quality</h2>
      </article>

      <article className="panel panel-half reveal" style={{ ["--delay" as string]: "0.18s" }}>
        <h2>Incidents par severite</h2>
      </article>

      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.22s" }}>
        <h2>Correlation map</h2>
      </article>

      <article className="panel panel-half reveal" style={{ ["--delay" as string]: "0.26s" }}>
        <h2>Exports & forensic snapshots</h2>
      </article>

      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.3s" }}>
        <h2>Operations journal</h2>
      </article>
    </>
  );
}