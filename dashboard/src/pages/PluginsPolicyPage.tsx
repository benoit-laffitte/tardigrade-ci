import type { PluginsPolicyPageProps } from "./types";
import { usePluginDomain } from "../hooks/plugins/domain";

// Renders the Plugins & Policy roadmap page with autonomous plugin-domain signals.
export function PluginsPolicyPage({
  adminRole,
  roleCapabilities,
  log,
  audit
}: Readonly<PluginsPolicyPageProps>) {
  const pluginDomain = usePluginDomain({
    adminRole,
    roleCapabilities,
    log,
    audit
  });

  return (
    <>
      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.02s" }}>
        <h2>Triage extensions & policy</h2>
        <p className="hint">Signaux prioritaires plugin/policy selon la maquette cible.</p>
        <div className="metrics-grid">
          <div className="metric-card">
            <p className="metric-label">Plugin failures</p>
            <p className="metric-value">
              {pluginDomain.pluginInventory.filter((plugin) => String(plugin.state).toLowerCase().includes("fail")).length}
            </p>
          </div>
          <div className="metric-card">
            <p className="metric-label">Policy violations</p>
            <p className="metric-value">5</p>
          </div>
          <div className="metric-card">
            <p className="metric-label">Drift detected</p>
            <p className="metric-value">{pluginDomain.pluginAuthorizationResult?.allowed ? 0 : 1}</p>
          </div>
          <div className="metric-card">
            <p className="metric-label">Coverage</p>
            <p className="metric-value">roadmap</p>
          </div>
        </div>
      </article>

      <article className="panel panel-third reveal" style={{ ["--delay" as string]: "0.06s" }}>
        <h2>Runbook actions</h2>
        <div className="list">
          <div className="list-item"><p className="item-subtitle">Disable failing plugin version</p></div>
          <div className="list-item"><p className="item-subtitle">Enforce deny-all fallback policy</p></div>
          <div className="list-item"><p className="item-subtitle">Trigger dry-run auth replay</p></div>
        </div>
      </article>

      <article className="panel panel-two-thirds reveal" style={{ ["--delay" as string]: "0.1s" }}>
        <h2>Plugin lifecycle health</h2>
        <p className="hint">Etat live du domaine plugin.</p>
        <div className="list">
          <div className="list-item">
            <p className="item-subtitle">Inventory: {pluginDomain.pluginInventory.length} plugin(s)</p>
          </div>
          <div className="list-item"><p className="item-subtitle">{pluginDomain.pluginAdminMessage || "Aucun message admin plugin."}</p></div>
          <div className="list-item"><p className="item-subtitle">{pluginDomain.pluginPolicyMessage || "Aucun message policy plugin."}</p></div>
        </div>
        <div className="actions">
          <button className="btn ghost" onClick={() => void pluginDomain.refreshPluginInventory()}>
            Refresh plugin inventory
          </button>
        </div>
      </article>

      <article className="panel panel-half reveal" style={{ ["--delay" as string]: "0.14s" }}>
        <h2>Policy enforcement coverage</h2>
        <p className="hint">Vue governance en demi-largeur.</p>
      </article>

      <article className="panel panel-half reveal" style={{ ["--delay" as string]: "0.18s" }}>
        <h2>Violations et drift</h2>
        <p className="hint">Timeline forensics roadmap.</p>
      </article>

      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.22s" }}>
        <h2>Capability governance</h2>
        <p className="hint">Edition capabilities avec garde-fous (roadmap).</p>
      </article>

      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.26s" }}>
        <h2>Inventory & provenance</h2>
        <div className="list">
          {pluginDomain.pluginInventory.length === 0 ? (
            <p className="hint">Aucun plugin charge.</p>
          ) : (
            pluginDomain.pluginInventory.slice(0, 8).map((plugin) => (
              <div className="list-item" key={`plugin-domain-${plugin.name}`}>
                <div>
                  <p className="item-title">{plugin.name}</p>
                  <p className="item-subtitle">State: {plugin.state}</p>
                  <p className="item-subtitle">Caps: {plugin.capabilities.join(", ") || "none"}</p>
                </div>
              </div>
            ))
          )}
        </div>
      </article>
    </>
  );
}