const jobForm = document.getElementById("jobForm");
const jobsList = document.getElementById("jobsList");
const buildsList = document.getElementById("buildsList");
const jobsCount = document.getElementById("jobsCount");
const buildsCount = document.getElementById("buildsCount");
const workersList = document.getElementById("workersList");
const workersCount = document.getElementById("workersCount");
const metricReclaims = document.getElementById("metricReclaims");
const metricRetries = document.getElementById("metricRetries");
const metricConflicts = document.getElementById("metricConflicts");
const metricDeadLetter = document.getElementById("metricDeadLetter");
const deadLetterList = document.getElementById("deadLetterList");
const deadLetterCount = document.getElementById("deadLetterCount");
const eventsList = document.getElementById("eventsList");
const eventCount = document.getElementById("eventCount");
const streamStatus = document.getElementById("streamStatus");
const logBox = document.getElementById("logBox");
const refreshBtn = document.getElementById("refreshBtn");
const createMessage = document.getElementById("createMessage");
const jobTemplate = document.getElementById("jobTemplate");
const buildTemplate = document.getElementById("buildTemplate");
const workerTemplate = document.getElementById("workerTemplate");
const stardateLabel = document.getElementById("stardateLabel");

const liveEvents = [];
let refreshTimer = null;
let streamConnected = false;

// Updates the decorative stardate indicator shown in the HUD strip.
function updateStardateLabel() {
  if (!stardateLabel) {
    return;
  }

  const now = new Date();
  const yearStart = new Date(now.getFullYear(), 0, 1);
  const dayOfYear = Math.floor((now - yearStart) / 86400000) + 1;
  const value = `${String(now.getFullYear()).slice(2)}.${String(dayOfYear).padStart(3, "0")}`;
  stardateLabel.textContent = `Stardate: ${value}`;
}

// Prepends one formatted line to the operator log console.
function log(message, kind = "info") {
  const now = new Date().toLocaleTimeString();
  const prefix = kind.toUpperCase().padEnd(5, " ");
  logBox.textContent = `[${now}] ${prefix} ${message}\n${logBox.textContent}`;
}

// Reflects realtime stream connectivity in the status chip.
function setStreamStatus(connected) {
  streamConnected = connected;
  if (!streamStatus) {
    return;
  }

  streamStatus.classList.toggle("connected", connected);
  streamStatus.classList.toggle("disconnected", !connected);
  streamStatus.textContent = connected ? "Realtime Online" : "Realtime Offline";
}

// Schedules a delayed full refresh and coalesces bursts of update signals.
function scheduleRefresh(delayMs = 120) {
  // Debounce avoids flooding the API when many events arrive in a burst.
  if (refreshTimer) {
    clearTimeout(refreshTimer);
  }
  refreshTimer = setTimeout(async () => {
    refreshTimer = null;
    await refreshAll();
  }, delayMs);
}

// Wrapper around fetch with unified JSON parsing and error shaping.
async function api(path, init = {}) {
  const response = await fetch(path, {
    headers: {
      "content-type": "application/json",
      ...init.headers,
    },
    ...init,
  });

  const isJson = (response.headers.get("content-type") || "").includes("application/json");
  const payload = isJson ? await response.json() : null;

  if (!response.ok) {
    const detail = payload ? JSON.stringify(payload) : response.statusText;
    throw new Error(`${response.status} ${detail}`);
  }

  return payload;
}

// Renders jobs list and binds run action for each job entry.
function renderJobs(jobs) {
  jobsList.innerHTML = "";
  jobsCount.textContent = String(jobs.length);

  if (jobs.length === 0) {
    jobsList.innerHTML = '<p class="hint">Aucun job pour le moment.</p>';
    return;
  }

  jobs.forEach((job) => {
    const fragment = jobTemplate.content.cloneNode(true);
    const node = fragment.querySelector(".job-item");
    node.querySelector(".item-title").textContent = job.name;
    node.querySelector(".item-subtitle").textContent = `${job.repository_url} | ${job.pipeline_path}`;
    node.querySelector(".run-btn").addEventListener("click", async () => {
      await runJob(job.id, job.name);
    });
    jobsList.appendChild(fragment);
  });
}

// Renders builds list with status badges and conditional cancel action.
function renderBuilds(builds) {
  buildsList.innerHTML = "";
  buildsCount.textContent = String(builds.length);

  if (builds.length === 0) {
    buildsList.innerHTML = '<p class="hint">Aucun build encore lance.</p>';
    return;
  }

  builds.forEach((build) => {
    const fragment = buildTemplate.content.cloneNode(true);
    const node = fragment.querySelector(".build-item");
    const statusNode = node.querySelector(".status");

    node.querySelector(".item-title").textContent = `Build ${build.id.slice(0, 8)}`;
    node.querySelector(".item-subtitle").textContent = `Job ${build.job_id.slice(0, 8)} | ${new Date(build.queued_at).toLocaleString()}`;

    statusNode.textContent = build.status;
    statusNode.classList.add(String(build.status).toLowerCase());

    const cancelBtn = node.querySelector(".cancel-btn");
    if (build.status === "Canceled" || build.status === "Success" || build.status === "Failed") {
      cancelBtn.disabled = true;
      cancelBtn.style.opacity = "0.4";
      cancelBtn.style.cursor = "default";
    } else {
      cancelBtn.addEventListener("click", async () => {
        await cancelBuild(build.id);
      });
    }

    buildsList.appendChild(fragment);
  });
}

// Renders workers list with activity and last-seen telemetry.
function renderWorkers(workers) {
  workersList.innerHTML = "";
  workersCount.textContent = String(workers.length);

  if (workers.length === 0) {
    workersList.innerHTML = '<p class="hint">Aucun worker visible.</p>';
    return;
  }

  workers.forEach((worker) => {
    const fragment = workerTemplate.content.cloneNode(true);
    const node = fragment.querySelector(".worker-item");
    const statusNode = node.querySelector(".worker-status");

    node.querySelector(".item-title").textContent = worker.id;
    node.querySelector(".item-subtitle").textContent = `Last seen ${new Date(worker.last_seen_at).toLocaleString()} | Active builds ${worker.active_builds}`;
    statusNode.textContent = worker.status;
    statusNode.classList.add(String(worker.status).toLowerCase());

    workersList.appendChild(fragment);
  });
}

// Renders runtime reliability counters in metric cards.
function renderMetrics(metrics) {
  if (!metrics) {
    return;
  }

  if (metricReclaims) {
    metricReclaims.textContent = String(metrics.reclaimed_total ?? 0);
  }
  if (metricRetries) {
    metricRetries.textContent = String(metrics.retry_requeued_total ?? 0);
  }
  if (metricConflicts) {
    metricConflicts.textContent = String(metrics.ownership_conflicts_total ?? 0);
  }
  if (metricDeadLetter) {
    metricDeadLetter.textContent = String(metrics.dead_letter_total ?? 0);
  }
}

// Renders dead-letter build list for operational follow-up.
function renderDeadLetterBuilds(builds) {
  if (!deadLetterList || !deadLetterCount) {
    return;
  }

  deadLetterList.innerHTML = "";
  deadLetterCount.textContent = String(builds.length);

  if (builds.length === 0) {
    deadLetterList.innerHTML = '<p class="hint">Aucun build dead-letter.</p>';
    return;
  }

  builds.forEach((build) => {
    const item = document.createElement("div");
    item.className = "list-item";
    item.innerHTML = `
      <div>
        <p class="item-title">Build ${build.id.slice(0, 8)}</p>
        <p class="item-subtitle">Job ${build.job_id.slice(0, 8)} | ${new Date(build.queued_at).toLocaleString()}</p>
      </div>
      <div class="actions">
        <span class="status failed">dead-letter</span>
      </div>
    `;
    deadLetterList.appendChild(item);
  });
}

// Renders bounded live event feed from SSE stream payloads.
function renderLiveEvents() {
  if (!eventsList || !eventCount) {
    return;
  }

  eventCount.textContent = String(liveEvents.length);
  eventsList.innerHTML = "";

  if (liveEvents.length === 0) {
    eventsList.innerHTML = '<p class="hint">Aucun evenement recu.</p>';
    return;
  }

  liveEvents.forEach((evt) => {
    const item = document.createElement("div");
    item.className = "list-item event-item";

    const statusClass = evt.severity === "ok" ? "success" : evt.severity === "error" ? "failed" : "pending";
    const stamp = evt.at ? new Date(evt.at).toLocaleTimeString() : new Date().toLocaleTimeString();

    item.innerHTML = `
      <div>
        <p class="item-title">${evt.kind || "event"}</p>
        <p class="item-subtitle">${stamp} | ${evt.message || ""}</p>
      </div>
      <div class="actions">
        <span class="status ${statusClass}">${evt.severity || "info"}</span>
      </div>
    `;

    eventsList.appendChild(item);
  });
}

// Inserts one live event into memory buffer and updates UI/logs.
function pushLiveEvent(evt) {
  // Keep a bounded in-memory feed so UI remains fast over long sessions.
  liveEvents.unshift(evt);
  if (liveEvents.length > 30) {
    liveEvents.length = 30;
  }

  renderLiveEvents();
  log(`${evt.kind || "event"}: ${evt.message || "update"}`, evt.severity || "info");
}

// Opens EventSource stream and wires lifecycle handlers for realtime mode.
function startEventStream() {
  if (typeof EventSource === "undefined") {
    log("EventSource non supporte, mode polling uniquement", "warn");
    return;
  }

  const source = new EventSource("/events");

  source.onopen = () => {
    setStreamStatus(true);
    log("Flux temps reel connecte", "ok");
  };

  source.onerror = () => {
    // Browser EventSource will auto-reconnect; we only reflect degraded state in UI.
    if (streamConnected) {
      log("Perte du flux temps reel, reconnexion en cours", "warn");
    }
    setStreamStatus(false);
  };

  source.onmessage = (event) => {
    try {
      const payload = JSON.parse(event.data);
      pushLiveEvent(payload);
      // Live payload gives signal, full refresh keeps all lists and metrics strongly consistent.
      scheduleRefresh(80);
    } catch (error) {
      log(`Evenement live invalide: ${error.message}`, "error");
    }
  };
}

// Pulls full dashboard state from API and refreshes all panels.
async function refreshAll() {
  try {
    const [jobsPayload, buildsPayload, workersPayload, metricsPayload, deadLetterPayload] = await Promise.all([
      api("/jobs"),
      api("/builds"),
      api("/workers"),
      api("/metrics"),
      api("/dead-letter-builds"),
    ]);
    renderJobs(jobsPayload.jobs);
    renderBuilds(buildsPayload.builds);
    renderWorkers(workersPayload.workers);
    renderMetrics(metricsPayload);
    renderDeadLetterBuilds(deadLetterPayload.builds || []);
  } catch (error) {
    log(`Echec du rafraichissement: ${error.message}`, "error");
  }
}

// Triggers run endpoint for one job and refreshes UI.
async function runJob(id, name) {
  try {
    const payload = await api(`/jobs/${id}/run`, { method: "POST", body: "{}" });
    log(`Build ${payload.build.id.slice(0, 8)} lance pour ${name}`, "ok");
    await refreshAll();
  } catch (error) {
    log(`Impossible de lancer le job ${name}: ${error.message}`, "error");
  }
}

// Triggers cancel endpoint for one build and refreshes UI.
async function cancelBuild(buildId) {
  try {
    await api(`/builds/${buildId}/cancel`, { method: "POST", body: "{}" });
    log(`Build ${buildId.slice(0, 8)} annule`, "ok");
    await refreshAll();
  } catch (error) {
    log(`Impossible d'annuler ${buildId.slice(0, 8)}: ${error.message}`, "error");
  }
}

jobForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const formData = new FormData(jobForm);
  const payload = Object.fromEntries(formData.entries());

  createMessage.textContent = "Creation en cours...";

  try {
    const response = await api("/jobs", {
      method: "POST",
      body: JSON.stringify(payload),
    });
    createMessage.textContent = `Job ${response.job.name} cree.`;
    log(`Nouveau job ${response.job.name} (${response.job.id.slice(0, 8)})`, "ok");
    jobForm.reset();
    await refreshAll();
  } catch (error) {
    createMessage.textContent = "Erreur de creation";
    log(`Creation job en erreur: ${error.message}`, "error");
  }
});

refreshBtn.addEventListener("click", async () => {
  await refreshAll();
  log("Donnees rafraichies", "info");
});

updateStardateLabel();
setInterval(updateStardateLabel, 60000);
setInterval(() => {
  if (!streamConnected) {
    // Polling fallback keeps dashboard usable when SSE is unavailable.
    refreshAll();
  }
}, 5000);

startEventStream();
renderLiveEvents();
log("Console initialisee", "ok");
await refreshAll();
