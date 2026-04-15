import { type AdminRole } from "./types";

interface DashboardHeaderProps {
  streamConnected: boolean;
  streamStatusText: string;
  healthStatus: "ok" | "degraded";
  adminRole: AdminRole;
  onAdminRoleChange: (role: AdminRole) => void;
  onRefresh: () => void;
}

// Renders top-level dashboard banner with stream/API state and role selector.
export function DashboardHeader({
  streamConnected,
  streamStatusText,
  healthStatus,
  adminRole,
  onAdminRoleChange,
  onRefresh
}: Readonly<DashboardHeaderProps>) {
  return (
    <header className="global-banner reveal" style={{ ["--delay" as string]: "0s" }}>
      <div className="banner-brand">
        <img className="banner-logo" src="/tardigrade-logo.png" alt="Tardigrade logo" />
        <div>
          <p className="eyebrow">Bridge Control Plane</p>
          <h1>Tardigrade Operations Console</h1>
        </div>
      </div>
      <div className="top-actions">
        <span className={`status-chip ${streamConnected ? "connected" : "disconnected"}`}>{streamStatusText}</span>
        <span className={`status-chip ${healthStatus === "ok" ? "connected" : "disconnected"}`}>
          API {healthStatus === "ok" ? "Healthy" : "Degraded"}
        </span>
        <label>
          <span>Role</span>
          <select
            name="admin_role"
            value={adminRole}
            onChange={(event) => onAdminRoleChange(event.target.value as AdminRole)}
          >
            <option value="viewer">viewer</option>
            <option value="operator">operator</option>
            <option value="admin">admin</option>
          </select>
        </label>
        <button className="btn btn-ghost" onClick={onRefresh} type="button">
          Synchroniser
        </button>
      </div>
    </header>
  );
}
