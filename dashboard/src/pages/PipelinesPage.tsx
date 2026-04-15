import type { PipelinesPageProps } from "./types";

// Renders the delivery-focused Pipelines page using currently available job/build APIs.
export function PipelinesPage({
  form,
  createMessage,
  jobs,
  builds,
  onCreateJob,
  onFormChange,
  onRunJob,
  onCancelBuild,
  formatDateTime
}: Readonly<PipelinesPageProps>) {
  return (
    <>
      <article className="panel panel-third panel-form reveal" style={{ ["--delay" as string]: "0.02s" }}>
        <h2>Nouveau Job</h2>
        <form className="form" onSubmit={onCreateJob}>
          <label>
            <span>Nom du job</span>
            <input
              name="name"
              placeholder="build-api"
              required
              value={form.name}
              onChange={(event) => onFormChange("name", event.target.value)}
            />
          </label>
          <label>
            <span>Depot git</span>
            <input
              name="repository_url"
              placeholder="https://example.com/project.git"
              required
              value={form.repository_url}
              onChange={(event) => onFormChange("repository_url", event.target.value)}
            />
          </label>
          <label>
            <span>Pipeline file</span>
            <input
              name="pipeline_path"
              placeholder="pipelines/api.yml"
              required
              value={form.pipeline_path}
              onChange={(event) => onFormChange("pipeline_path", event.target.value)}
            />
          </label>
          <button type="submit" className="btn btn-primary">
            POST /jobs
          </button>
        </form>
        <p className="hint">{createMessage}</p>
      </article>

      <article className="panel panel-third reveal" style={{ ["--delay" as string]: "0.12s" }}>
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
                <div className="actions">
                  <button className="btn btn-small btn-secondary" type="button" onClick={() => onRunJob(job.id, job.name)}>
                    POST /jobs/{"{id}"}/run
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </article>

      <article className="panel panel-full reveal" style={{ ["--delay" as string]: "0.22s" }}>
        <div className="panel-head">
          <h2>Builds</h2>
          <span className="pill">{builds.length}</span>
        </div>
        <div className="list">
          {builds.length === 0 ? (
            <p className="hint">Aucun build encore lance.</p>
          ) : (
            builds.map((build) => {
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
                      onClick={() => onCancelBuild(build.id)}
                      style={isFinal ? { opacity: 0.4, cursor: "default" } : undefined}
                    >
                      POST /builds/{"{id}"}/cancel
                    </button>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </article>
    </>
  );
}