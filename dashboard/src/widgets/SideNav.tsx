import { type DashboardNavItem, type DashboardPage } from "./types";

interface SideNavProps {
  navItems: DashboardNavItem[];
  activePage: DashboardPage;
  stardate: string;
  onSelectPage: (page: DashboardPage) => void;
}

// Renders the left-side dashboard navigation and contextual footer metadata.
export function SideNav({ navItems, activePage, stardate, onSelectPage }: Readonly<SideNavProps>) {
  return (
    <aside className="sidenav reveal" style={{ ["--delay" as string]: "0s" }}>
      <div className="brand">
        <p className="eyebrow">Navigation</p>
        <h1>Screens</h1>
        <p className="subtitle">Disposition alignee sur la maquette produit.</p>
      </div>

      <nav className="page-nav" aria-label="Pages metier">
        {navItems.map((item, index) => (
          <button
            key={item.id}
            type="button"
            className={`page-tab ${activePage === item.id ? "active" : ""}`}
            onClick={() => onSelectPage(item.id)}
          >
            {index + 1}. {item.label}
          </button>
        ))}
      </nav>

      <div className="sidenav-foot">
        <p>
          <strong>Landing:</strong> Pipelines
        </p>
        <p>
          <strong>Stardate:</strong> {stardate}
        </p>
      </div>
    </aside>
  );
}
