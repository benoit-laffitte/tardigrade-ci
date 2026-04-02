import { gql, useApolloClient } from "@apollo/client";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type BuildStatus = "Pending" | "Running" | "Success" | "Failed" | "Canceled";
type EventSeverity = "ok" | "error" | "warn" | "info";

interface Job {
  id: string;
  name: string;
  repository_url: string;
  pipeline_path: string;
  created_at: string;
}

interface Build {
  id: string;
  job_id: string;
  status: BuildStatus;
  queued_at: string;
  started_at?: string | null;
  finished_at?: string | null;
  logs: string[];
}

interface Worker {
  id: string;
  active_builds: number;
  status: string;
  last_seen_at: string;
}

interface RuntimeMetrics {
  reclaimed_total: number;
  retry_requeued_total: number;
  ownership_conflicts_total: number;
  dead_letter_total: number;
}

interface LiveEvent {
  kind?: string;
  message?: string;
  severity?: EventSeverity;
  at?: string;
}

interface DashboardSnapshot {
  jobs: Job[];
  builds: Build[];
  workers: Worker[];
  metrics: RuntimeMetrics | null;
  dead_letter_builds: Build[];
}

interface DashboardSnapshotResponse {
  dashboard_snapshot: DashboardSnapshot;
}

interface CreateJobResponse {
  create_job: Pick<Job, "id" | "name">;
}

interface RunJobResponse {
  run_job: Pick<Build, "id">;
}

interface CancelBuildResponse {
  cancel_build: Pick<Build, "id">;
}

interface CreateJobInput {
  name: string;
  repository_url: string;
  pipeline_path: string;
}

type ScmProvider = "github" | "gitlab";

interface WebhookSecurityInput {
  repository_url: string;
  provider: ScmProvider;
  secret: string;
  allowed_ips_text: string;
}

const DASHBOARD_SNAPSHOT_QUERY = gql`
  query DashboardSnapshot {
    dashboard_snapshot {
      jobs {
        id
        name
        repository_url
        pipeline_path
        created_at
      }
      builds {
        id
        job_id
        status
        queued_at
        started_at
        finished_at
        logs
      }
      workers {
        id
        active_builds
        status
        last_seen_at
      }
      metrics {
        reclaimed_total
        retry_requeued_total
        ownership_conflicts_total
        dead_letter_total
      }
      dead_letter_builds {
        id
        job_id
        status
        queued_at
      }
    }
  }
`;

const CREATE_JOB_MUTATION = gql`
  mutation CreateJob($input: GqlCreateJobInput!) {
    create_job(input: $input) {
      id
      name
    }
  }
`;

const RUN_JOB_MUTATION = gql`
  mutation RunJob($jobId: ID!) {
    run_job(jobId: $jobId) {
      id
    }
  }
`;

const CANCEL_BUILD_MUTATION = gql`
  mutation CancelBuild($buildId: ID!) {
    cancel_build(buildId: $buildId) {
      id
    }
  }
`;

// Maps incoming SSE severity values to dashboard badge classes.
function severityToStatusClass(severity?: EventSeverity): "success" | "failed" | "pending" {
  if (severity === "ok") {
    return "success";
  }
  if (severity === "error") {
    return "failed";
  }
  return "pending";
}

// Formats timestamps in local time while handling missing values.
function formatDateTime(value?: string | null): string {
  if (!value) {
    return "-";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "-";
  }
  return date.toLocaleString();
}

// Formats timestamps in local time (short variant for event feed).
function formatTime(value?: string | null): string {
  if (!value) {
    return new Date().toLocaleTimeString();
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return new Date().toLocaleTimeString();
  }
  return date.toLocaleTimeString();
}

// Normalizes allowlist text input into unique trimmed IP entries.
function normalizeAllowlistInput(raw: string): string[] {
  const values = raw
    .split(/[,\n]/)
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  return Array.from(new Set(values));
}

// Returns the display stardate used in the top HUD strip.
function stardateValue(now: Date): string {
  const yearStart = new Date(now.getFullYear(), 0, 1);
  const dayOfYear = Math.floor((now.getTime() - yearStart.getTime()) / 86400000) + 1;
  return `${String(now.getFullYear()).slice(2)}.${String(dayOfYear).padStart(3, "0")}`;
}

export function App() {
  const client = useApolloClient();
  const refreshTimerRef = useRef<number | null>(null);

  const [streamConnected, setStreamConnected] = useState(false);
  const [snapshot, setSnapshot] = useState<DashboardSnapshot>({
    jobs: [],
    builds: [],
    workers: [],
    metrics: null,
    dead_letter_builds: []
  });
  const [liveEvents, setLiveEvents] = useState<LiveEvent[]>([]);
  const [logs, setLogs] = useState("");
  const [createMessage, setCreateMessage] = useState("");
  const [form, setForm] = useState<CreateJobInput>({
    name: "",
    repository_url: "",
    pipeline_path: ""
  });
  const [webhookForm, setWebhookForm] = useState<WebhookSecurityInput>({
    repository_url: "",
    provider: "github",
    secret: "",
    allowed_ips_text: ""
  });
  const [webhookMessage, setWebhookMessage] = useState("");
  const [showWebhookSecret, setShowWebhookSecret] = useState(false);
  const [knownWebhookConfigs, setKnownWebhookConfigs] = useState<Set<string>>(new Set());
  const [stardate, setStardate] = useState(() => stardateValue(new Date()));

  // Prepends one log line to keep operator feedback visible.
  const log = useCallback((message: string, kind: string = "info") => {
    const now = new Date().toLocaleTimeString();
    const prefix = kind.toUpperCase().padEnd(5, " ");
    setLogs((prev) => `[${now}] ${prefix} ${message}\n${prev}`);
  }, []);

  // Pulls snapshot data from GraphQL and updates all dashboard panels.
  const refreshAll = useCallback(async () => {
    try {
      const { data } = await client.query<DashboardSnapshotResponse>({
        query: DASHBOARD_SNAPSHOT_QUERY,
        fetchPolicy: "network-only"
      });
      setSnapshot(data.dashboard_snapshot);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      log(`Echec du rafraichissement: ${message}`, "error");
    }
  }, [client, log]);

  // Debounces refresh calls to avoid overloading API on event bursts.
  const scheduleRefresh = useCallback(
    (delayMs: number = 120) => {
      if (refreshTimerRef.current) {
        globalThis.clearTimeout(refreshTimerRef.current);
      }
      refreshTimerRef.current = globalThis.setTimeout(() => {
        refreshTimerRef.current = null;
        void refreshAll();
      }, delayMs);
    },
    [refreshAll]
  );

  // Stores one event in memory, rerenders feed, and writes an operator log.
  const pushLiveEvent = useCallback(
    (evt: LiveEvent) => {
      setLiveEvents((previous) => {
        const next = [evt, ...previous];
        return next.slice(0, 30);
      });
      log(`${evt.kind ?? "event"}: ${evt.message ?? "update"}`, evt.severity ?? "info");
    },
    [log]
  );

  // Executes one job and refreshes the dashboard after mutation completes.
  const runJob = useCallback(
    async (jobId: string, name: string) => {
      try {
        const { data } = await client.mutate<RunJobResponse>({
          mutation: RUN_JOB_MUTATION,
          variables: { jobId }
        });
        if (!data?.run_job.id) {
          throw new Error("run_job did not return a build id");
        }
        log(`Build ${data.run_job.id.slice(0, 8)} lance pour ${name}`, "ok");
        await refreshAll();
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        log(`Impossible de lancer le job ${name}: ${message}`, "error");
      }
    },
    [client, log, refreshAll]
  );

  // Cancels one build and refreshes dashboard state.
  const cancelBuild = useCallback(
    async (buildId: string) => {
      try {
        await client.mutate<CancelBuildResponse>({
          mutation: CANCEL_BUILD_MUTATION,
          variables: { buildId }
        });
        log(`Build ${buildId.slice(0, 8)} annule`, "ok");
        await refreshAll();
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        log(`Impossible d'annuler ${buildId.slice(0, 8)}: ${message}`, "error");
      }
    },
    [client, log, refreshAll]
  );

  // Creates a job from form payload and refreshes dashboard data.
  const createJob = useCallback(
    async (event: { preventDefault: () => void }) => {
      event.preventDefault();
      setCreateMessage("Creation en cours...");

      try {
        const { data } = await client.mutate<CreateJobResponse>({
          mutation: CREATE_JOB_MUTATION,
          variables: { input: form }
        });

        if (!data?.create_job) {
          throw new Error("create_job did not return payload");
        }

        setCreateMessage(`Job ${data.create_job.name} cree.`);
        log(`Nouveau job ${data.create_job.name} (${data.create_job.id.slice(0, 8)})`, "ok");
        setForm({ name: "", repository_url: "", pipeline_path: "" });
        await refreshAll();
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        setCreateMessage("Erreur de creation");
        log(`Creation job en erreur: ${message}`, "error");
      }
    },
    [client, form, log, refreshAll]
  );

  // Saves webhook security settings for one repository/provider pair.
  const saveWebhookSecurityConfig = useCallback(
    async (event: { preventDefault: () => void }) => {
      event.preventDefault();
      const repository = webhookForm.repository_url.trim();
      const secret = webhookForm.secret.trim();
      const configKey = `${repository.toLowerCase()}::${webhookForm.provider}`;

      if (!repository || !secret) {
        setWebhookMessage("Parametres invalides: repository et secret requis.");
        log("Configuration webhook invalide: repository/secret manquant", "warn");
        return;
      }

      if (knownWebhookConfigs.has(configKey)) {
        const confirmed = globalThis.confirm(
          "Une configuration existe deja pour ce repository/provider. Confirmer l'ecrasement ?"
        );
        if (!confirmed) {
          setWebhookMessage("Ecrasement annule.");
          return;
        }
      }

      const payload = {
        repository_url: repository,
        provider: webhookForm.provider,
        secret,
        allowed_ips: normalizeAllowlistInput(webhookForm.allowed_ips_text)
      };

      try {
        const response = await fetch("/scm/webhook-security/configs", {
          method: "POST",
          headers: {
            "content-type": "application/json"
          },
          body: JSON.stringify(payload)
        });

        if (response.status === 204) {
          setWebhookMessage("Configuration webhook enregistree.");
          setKnownWebhookConfigs((previous) => new Set(previous).add(configKey));
          log(`Webhook security sauvegardee pour ${repository} (${webhookForm.provider})`, "ok");
          return;
        }

        if (response.status === 400) {
          setWebhookMessage("Configuration invalide.");
          log("Rejet de configuration webhook: payload invalide", "warn");
          return;
        }

        if (response.status === 403) {
          setWebhookMessage("Configuration refusee (forbidden).");
          log("Configuration webhook refusee (403)", "error");
          return;
        }

        setWebhookMessage("Erreur interne lors de la sauvegarde webhook.");
        log(`Configuration webhook en echec: HTTP ${response.status}`, "error");
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        setWebhookMessage("Erreur reseau lors de la sauvegarde webhook.");
        log(`Configuration webhook en echec: ${message}`, "error");
      }
    },
    [knownWebhookConfigs, log, webhookForm]
  );

  // Initializes dashboard data and baseline log once on first mount.
  useEffect(() => {
    log("Console initialisee", "ok");
    void refreshAll();
  }, [log, refreshAll]);

  // Keeps stardate indicator updated each minute.
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      setStardate(stardateValue(new Date()));
    }, 60000);
    return () => globalThis.clearInterval(id);
  }, []);

  // Polling fallback ensures updates continue while SSE is disconnected.
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      if (!streamConnected) {
        void refreshAll();
      }
    }, 5000);
    return () => globalThis.clearInterval(id);
  }, [streamConnected, refreshAll]);

  // Opens SSE stream and wires realtime events to logs + snapshot refresh.
  useEffect(() => {
    if (globalThis.EventSource === undefined) {
      log("EventSource non supporte, mode polling uniquement", "warn");
      return;
    }

    const source = new EventSource("/events");

    source.onopen = () => {
      setStreamConnected(true);
      log("Flux temps reel connecte", "ok");
    };

    source.onerror = () => {
      setStreamConnected((previous) => {
        if (previous) {
          log("Perte du flux temps reel, reconnexion en cours", "warn");
        }
        return false;
      });
    };

    source.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data) as LiveEvent;
        pushLiveEvent(payload);
        scheduleRefresh(80);
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        log(`Evenement live invalide: ${message}`, "error");
      }
    };

    return () => {
      source.close();
    };
  }, [log, pushLiveEvent, scheduleRefresh]);

  // Clears any pending debounced refresh timer on unmount.
  useEffect(() => {
    return () => {
      if (refreshTimerRef.current) {
        globalThis.clearTimeout(refreshTimerRef.current);
      }
    };
  }, []);

  // Derived text for stream status chip.
  const streamStatusText = useMemo(
    () => (streamConnected ? "Realtime Online" : "Realtime Offline"),
    [streamConnected]
  );

  return (
    <>
      <div className="bg-orb orb-1"></div>
      <div className="bg-orb orb-2"></div>
      <div className="bg-orb orb-3"></div>
      <div className="bg-grid"></div>
      <div className="bg-scanline"></div>

      <main className="shell">
        <section className="hud-strip reveal" style={{ ["--delay" as string]: "0s" }}>
          <span>Deck: CI-01</span>
          <span>Channel: Build Control</span>
          <span>Stardate: {stardate}</span>
        </section>

        <header className="hero">
          <div className="hero-copy-wrap">
            <div className="logo-shell" aria-hidden="true">
              <img className="tardi-logo" src="/tardigrade-logo.png" alt="" />
            </div>
            <div className="hero-copy">
              <p className="eyebrow">Bridge Control Plane</p>
              <h1>Tardigrade CI Console</h1>
              <p className="subtitle">
                Interface tactique pour creer des jobs, lancer des builds et piloter les executions en temps reel.
              </p>
            </div>
          </div>
          <div className="hero-actions">
            <div className={`status-chip ${streamConnected ? "connected" : "disconnected"}`}>
              {streamStatusText}
            </div>
            <button className="btn btn-ghost" onClick={() => void refreshAll()} type="button">
              Synchroniser
            </button>
          </div>
        </header>

        <section className="grid">
          <article className="panel panel-form reveal" style={{ ["--delay" as string]: "0.02s" }}>
            <h2>Nouveau Job</h2>
            <form className="form" onSubmit={(event) => void createJob(event)}>
              <label>
                <span>Nom du job</span>
                <input
                  name="name"
                  placeholder="build-api"
                  required
                  value={form.name}
                  onChange={(event) => setForm((prev) => ({ ...prev, name: event.target.value }))}
                />
              </label>
              <label>
                <span>Depot git</span>
                <input
                  name="repository_url"
                  placeholder="https://example.com/project.git"
                  required
                  value={form.repository_url}
                  onChange={(event) => setForm((prev) => ({ ...prev, repository_url: event.target.value }))}
                />
              </label>
              <label>
                <span>Pipeline file</span>
                <input
                  name="pipeline_path"
                  placeholder="pipelines/api.yml"
                  required
                  value={form.pipeline_path}
                  onChange={(event) => setForm((prev) => ({ ...prev, pipeline_path: event.target.value }))}
                />
              </label>
              <button type="submit" className="btn btn-primary">
                Initier le job
              </button>
            </form>
            <p className="hint">{createMessage}</p>
          </article>

          <article className="panel panel-form reveal" style={{ ["--delay" as string]: "0.06s" }}>
            <h2>SCM Webhook Security</h2>
            <form className="form" onSubmit={(event) => void saveWebhookSecurityConfig(event)}>
              <label>
                <span>Repository URL</span>
                <input
                  name="webhook_repository_url"
                  placeholder="https://example.com/repo.git"
                  required
                  value={webhookForm.repository_url}
                  onChange={(event) =>
                    setWebhookForm((previous) => ({ ...previous, repository_url: event.target.value }))
                  }
                />
              </label>
              <label>
                <span>Provider</span>
                <select
                  name="webhook_provider"
                  value={webhookForm.provider}
                  onChange={(event) =>
                    setWebhookForm((previous) => ({
                      ...previous,
                      provider: event.target.value as ScmProvider
                    }))
                  }
                >
                  <option value="github">github</option>
                  <option value="gitlab">gitlab</option>
                </select>
              </label>
              <label>
                <span>Secret</span>
                <input
                  name="webhook_secret"
                  type={showWebhookSecret ? "text" : "password"}
                  placeholder="super-secret"
                  required
                  value={webhookForm.secret}
                  onChange={(event) => setWebhookForm((previous) => ({ ...previous, secret: event.target.value }))}
                />
              </label>
              <div className="actions">
                <button
                  className="btn btn-small btn-secondary"
                  type="button"
                  onClick={() => setShowWebhookSecret((previous) => !previous)}
                >
                  {showWebhookSecret ? "Masquer" : "Reveler"}
                </button>
              </div>
              <label>
                <span>IP allowlist (comma/newline)</span>
                <textarea
                  name="webhook_allowed_ips"
                  placeholder="203.0.113.10, 198.51.100.20"
                  value={webhookForm.allowed_ips_text}
                  onChange={(event) =>
                    setWebhookForm((previous) => ({ ...previous, allowed_ips_text: event.target.value }))
                  }
                />
              </label>
              <div className="actions">
                <button type="submit" className="btn btn-primary">
                  Enregistrer
                </button>
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={() =>
                    setWebhookForm({
                      repository_url: "",
                      provider: "github",
                      secret: "",
                      allowed_ips_text: ""
                    })
                  }
                >
                  Effacer
                </button>
              </div>
            </form>
            <p className="hint">{webhookMessage}</p>
          </article>

          <article className="panel reveal" style={{ ["--delay" as string]: "0.12s" }}>
            <div className="panel-head">
              <h2>Jobs</h2>
              <span className="pill">{snapshot.jobs.length}</span>
            </div>
            <div className="list">
              {snapshot.jobs.length === 0 ? (
                <p className="hint">Aucun job pour le moment.</p>
              ) : (
                snapshot.jobs.map((job) => (
                  <div className="list-item job-item" key={job.id}>
                    <div>
                      <p className="item-title">{job.name}</p>
                      <p className="item-subtitle">
                        {job.repository_url} | {job.pipeline_path}
                      </p>
                    </div>
                    <div className="actions">
                      <button className="btn btn-small btn-secondary" type="button" onClick={() => void runJob(job.id, job.name)}>
                        Run
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
          </article>

          <article className="panel reveal" style={{ ["--delay" as string]: "0.22s" }}>
            <div className="panel-head">
              <h2>Builds</h2>
              <span className="pill">{snapshot.builds.length}</span>
            </div>
            <div className="list">
              {snapshot.builds.length === 0 ? (
                <p className="hint">Aucun build encore lance.</p>
              ) : (
                snapshot.builds.map((build) => {
                  const isFinal =
                    build.status === "Canceled" || build.status === "Success" || build.status === "Failed";

                  return (
                    <div className="list-item build-item" key={build.id}>
                      <div>
                        <p className="item-title">Build {build.id.slice(0, 8)}</p>
                        <p className="item-subtitle">
                          Job {build.job_id.slice(0, 8)} | {formatDateTime(build.queued_at)}
                        </p>
                      </div>
                      <div className="actions">
                        <span className={`status ${String(build.status).toLowerCase()}`}>{build.status}</span>
                        <button
                          className="btn btn-small btn-warning"
                          type="button"
                          disabled={isFinal}
                          onClick={() => void cancelBuild(build.id)}
                          style={isFinal ? { opacity: 0.4, cursor: "default" } : undefined}
                        >
                          Cancel
                        </button>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </article>

          <article className="panel reveal" style={{ ["--delay" as string]: "0.28s" }}>
            <div className="panel-head">
              <h2>Workers</h2>
              <span className="pill">{snapshot.workers.length}</span>
            </div>
            <div className="list">
              {snapshot.workers.length === 0 ? (
                <p className="hint">Aucun worker visible.</p>
              ) : (
                snapshot.workers.map((worker) => (
                  <div className="list-item worker-item" key={worker.id}>
                    <div>
                      <p className="item-title">{worker.id}</p>
                      <p className="item-subtitle">
                        Last seen {formatDateTime(worker.last_seen_at)} | Active builds {worker.active_builds}
                      </p>
                    </div>
                    <div className="actions">
                      <span className={`status worker-status ${String(worker.status).toLowerCase()}`}>{worker.status}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </article>

          <article className="panel panel-metrics reveal" style={{ ["--delay" as string]: "0.3s" }}>
            <div className="panel-head">
              <h2>Runtime Metrics</h2>
              <span className="pill">live</span>
            </div>
            <div className="metrics-grid">
              <div className="metric-card">
                <p className="metric-label">Reclaims</p>
                <p className="metric-value">{snapshot.metrics?.reclaimed_total ?? 0}</p>
              </div>
              <div className="metric-card">
                <p className="metric-label">Retry Requeues</p>
                <p className="metric-value">{snapshot.metrics?.retry_requeued_total ?? 0}</p>
              </div>
              <div className="metric-card">
                <p className="metric-label">Ownership Conflicts</p>
                <p className="metric-value">{snapshot.metrics?.ownership_conflicts_total ?? 0}</p>
              </div>
              <div className="metric-card">
                <p className="metric-label">Dead-letter</p>
                <p className="metric-value">{snapshot.metrics?.dead_letter_total ?? 0}</p>
              </div>
            </div>
          </article>

          <article className="panel reveal" style={{ ["--delay" as string]: "0.31s" }}>
            <div className="panel-head">
              <h2>Dead-letter Builds</h2>
              <span className="pill">{snapshot.dead_letter_builds.length}</span>
            </div>
            <div className="list">
              {snapshot.dead_letter_builds.length === 0 ? (
                <p className="hint">Aucun build dead-letter.</p>
              ) : (
                snapshot.dead_letter_builds.map((build) => (
                  <div className="list-item" key={build.id}>
                    <div>
                      <p className="item-title">Build {build.id.slice(0, 8)}</p>
                      <p className="item-subtitle">
                        Job {build.job_id.slice(0, 8)} | {formatDateTime(build.queued_at)}
                      </p>
                    </div>
                    <div className="actions">
                      <span className="status failed">dead-letter</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </article>

          <article className="panel panel-events reveal" style={{ ["--delay" as string]: "0.315s" }}>
            <div className="panel-head">
              <h2>Evenements Live</h2>
              <span className="pill">{liveEvents.length}</span>
            </div>
            <div className="list events-list">
              {liveEvents.length === 0 ? (
                <p className="hint">Aucun evenement recu.</p>
              ) : (
                liveEvents.map((evt, index) => (
                  <div className="list-item event-item" key={`${evt.kind ?? "event"}-${evt.at ?? "now"}-${index}`}>
                    <div>
                      <p className="item-title">{evt.kind ?? "event"}</p>
                      <p className="item-subtitle">
                        {formatTime(evt.at)} | {evt.message ?? ""}
                      </p>
                    </div>
                    <div className="actions">
                      <span className={`status ${severityToStatusClass(evt.severity)}`}>{evt.severity ?? "info"}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </article>
        </section>

        <section className="panel console reveal" style={{ ["--delay" as string]: "0.32s" }}>
          <h2>Journal de bord</h2>
          <pre aria-live="polite">{logs}</pre>
        </section>
      </main>
    </>
  );
}
