import { ApiCoveragePanel } from "./widgets/ApiCoveragePanel";
import { ConsoleWidget } from "./widgets/ConsoleWidget";
import { DashboardHeader } from "./widgets/DashboardHeader";
import { AdministrationPage } from "./pages/AdministrationPage";
import { ObservabilityPage } from "./pages/ObservabilityPage";
import { OverviewPage } from "./pages/OverviewPage";
import { PipelinesPage } from "./pages/PipelinesPage";
import { PluginsPolicyPage } from "./pages/PluginsPolicyPage";
import { ScmSecurityPage } from "./pages/ScmSecurityPage";
import { WorkersPage } from "./pages/WorkersPage";
import { SideNav } from "./widgets/SideNav";
import { useController } from "./hooks/core/controller";
import type { Worker } from "./hooks/dashboardTypes";

// Composes dashboard widgets while delegating business logic to the controller hook.
export function App() {
  const {
    streamConnected,
    activePage,
    setActivePage,
    healthStatus,
    adminRole,
    setAdminRole,
    logs,
    createMessage,
    form,
    setForm,
    snapshot,
    stardate,
    activeCoverage,
    streamStatusText,
    currentPage,
    workersReadOnlySummary,
    recentExecutionBuilds,
    scmSecurityReadOnlySummary,
    adminActivity,
    roleCapabilities,
    setSnapshot,
    log,
    audit,
    deliverySuccessRatio,
    buildStatusSummary,
    refreshAll,
    runJob,
    cancelBuild,
    createJob,
    formatDateTime,
    DASHBOARD_NAV_ITEMS
  } = useController();

  let activePageContent: React.JSX.Element;

  if (activePage === "pipelines") {
    activePageContent = (
      <PipelinesPage
        form={form}
        createMessage={createMessage}
        jobs={snapshot.jobs}
        builds={snapshot.builds}
        onCreateJob={createJob}
        onFormChange={(field, value) => setForm((prev) => ({ ...prev, [field]: value }))}
        onRunJob={(jobId, name) => {
          runJob(jobId, name);
        }}
        onCancelBuild={(buildId) => {
          cancelBuild(buildId);
        }}
        formatDateTime={formatDateTime}
      />
    );
  } else if (activePage === "overview") {
    activePageContent = (
      <OverviewPage
        jobs={snapshot.jobs}
        builds={snapshot.builds}
        healthStatus={healthStatus}
        deliverySuccessRatio={deliverySuccessRatio}
        buildStatusSummary={buildStatusSummary}
        formatDateTime={formatDateTime}
      />
    );
  } else if (activePage === "workers") {
    activePageContent = (
      <WorkersPage
        workersReadOnlySummary={workersReadOnlySummary}
        recentExecutionBuilds={recentExecutionBuilds}
        adminRole={adminRole}
        roleCapabilities={roleCapabilities}
        setWorkersSnapshot={(workers: Worker[]) => setSnapshot((previous) => ({ ...previous, workers }))}
        log={log}
        audit={audit}
        refreshAll={refreshAll}
        formatDateTime={formatDateTime}
      />
    );
  } else if (activePage === "scm-security") {
    activePageContent = (
      <ScmSecurityPage
        scmSecurityReadOnlySummary={scmSecurityReadOnlySummary}
        adminRole={adminRole}
        roleCapabilities={roleCapabilities}
        log={log}
        audit={audit}
        refreshAll={refreshAll}
      />
    );
  } else if (activePage === "plugins-policy") {
    activePageContent = (
      <PluginsPolicyPage
        adminRole={adminRole}
        roleCapabilities={roleCapabilities}
        log={log}
        audit={audit}
      />
    );
  } else if (activePage === "observability") {
    activePageContent = <ObservabilityPage />;
  } else {
    activePageContent = (
      <AdministrationPage
        adminActivity={adminActivity}
        roleCapabilities={roleCapabilities}
      />
    );
  }

  return (
    <>
      <div className="bg-orb orb-1"></div>
      <div className="bg-orb orb-2"></div>
      <div className="bg-orb orb-3"></div>
      <div className="bg-grid"></div>
      <div className="bg-scanline"></div>

      <main className="shell shell-mockup">
        <DashboardHeader
          streamConnected={streamConnected}
          streamStatusText={streamStatusText}
          healthStatus={healthStatus}
          adminRole={adminRole}
          onAdminRoleChange={setAdminRole}
          onRefresh={() => {
            refreshAll();
          }}
        />

        <div className="layout">
          <SideNav
            navItems={DASHBOARD_NAV_ITEMS}
            activePage={activePage}
            stardate={stardate}
            onSelectPage={setActivePage}
          />

          <div className="content">
            <header className="topbar reveal" style={{ ["--delay" as string]: "0.01s" }}>
              <div>
                <p className="eyebrow">{currentPage.kicker}</p>
                <h2>{currentPage.title}</h2>
              </div>
              <span className="pill">{activeCoverage}</span>
            </header>

            <section className="grid">
              <ConsoleWidget logs={logs} />
              <ApiCoveragePanel activeCoverage={activeCoverage} />
              {activePageContent}
            </section>
          </div>
        </div>
      </main>
    </>
  );
}
