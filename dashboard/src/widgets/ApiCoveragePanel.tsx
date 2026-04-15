interface ApiCoveragePanelProps {
  activeCoverage: "full" | "partial" | "roadmap";
}

// Renders API coverage context so operators understand page capability boundaries.
export function ApiCoveragePanel({ activeCoverage }: Readonly<ApiCoveragePanelProps>) {
  return (
    <section className="panel panel-full reveal api-coverage-panel" style={{ ["--delay" as string]: "0.015s" }}>
      <div className="panel-head">
        <h2>Perimetre API reel</h2>
        <span className="pill">{activeCoverage}</span>
      </div>
      <p className="hint">
        Endpoints disponibles: GET /health, POST /jobs, GET /jobs, POST /jobs/{"{id}"}/run,
        POST /builds/{"{id}"}/cancel, GET /builds.
      </p>
    </section>
  );
}
