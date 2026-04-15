import type { EventSeverity, LiveEvent, ObservabilityFilter } from "./dashboardTypes";

// Maps incoming SSE severity values to dashboard badge classes.
export function severityToStatusClass(severity?: EventSeverity): "success" | "failed" | "pending" {
  if (severity === "ok") {
    return "success";
  }
  if (severity === "error") {
    return "failed";
  }
  return "pending";
}

// Formats timestamps in local time while handling missing values.
export function formatDateTime(value?: string | null): string {
  if (!value) {
    return "-";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "-";
  }
  return date.toLocaleString();
}

// Formats timestamps in local time using a compact time-only representation.
export function formatTime(value?: string | null): string {
  if (!value) {
    return new Date().toLocaleTimeString();
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return new Date().toLocaleTimeString();
  }
  return date.toLocaleTimeString();
}

// Normalizes comma/newline-delimited text into unique trimmed entries.
export function normalizeDelimitedInput(raw: string): string[] {
  const values = raw
    .split(/[,\n]/)
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  return Array.from(new Set(values));
}

// Normalizes allowlist text input into unique trimmed IP entries.
export function normalizeAllowlistInput(raw: string): string[] {
  return normalizeDelimitedInput(raw);
}

// Normalizes branch text input into unique trimmed branch names.
export function normalizeBranchesInput(raw: string): string[] {
  return normalizeDelimitedInput(raw);
}

// Computes missing capabilities from required/granted sets for policy explainability.
export function missingCapabilities(required: string[], granted: string[]): string[] {
  return required.filter((capability) => !granted.includes(capability));
}

// Converts event rows to CSV text for incident handoff exports.
export function observabilityEventsToCsv(events: LiveEvent[]): string {
  const header = ["at", "kind", "severity", "message", "job_id", "build_id", "worker_id"];
  const escape = (value?: string) => `"${(value ?? "").replaceAll("\"", "\"\"")}"`;
  const rows = events.map((event) => [
    event.at ?? "",
    event.kind ?? "",
    event.severity ?? "",
    event.message ?? "",
    event.job_id ?? "",
    event.build_id ?? "",
    event.worker_id ?? ""
  ]);

  return [header, ...rows].map((row) => row.map((value) => escape(String(value))).join(",")).join("\n");
}

// Triggers browser download for one text payload using the provided mime type.
export function downloadTextPayload(filename: string, content: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

// Checks whether the filter resource id matches any event resource identifier.
export function matchesEventResource(event: LiveEvent, resourceId: string): boolean {
  if (!resourceId) {
    return true;
  }
  const resource = `${event.job_id ?? ""} ${event.build_id ?? ""} ${event.worker_id ?? ""}`.toLowerCase();
  return resource.includes(resourceId);
}

// Checks whether one event falls inside the configured observability time window.
export function matchesEventWindow(event: LiveEvent, windowMinutes: number, nowMs: number): boolean {
  if (!(Number.isFinite(windowMinutes) && windowMinutes > 0 && windowMinutes < 100000)) {
    return true;
  }

  const eventTs = event.at ? new Date(event.at).getTime() : nowMs;
  if (Number.isNaN(eventTs)) {
    return true;
  }

  const ageMinutes = (nowMs - eventTs) / 60000;
  return ageMinutes <= windowMinutes;
}

// Applies one observability filter set to one live event.
export function matchesObservabilityFilter(
  event: LiveEvent,
  filter: ObservabilityFilter,
  nowMs: number
): boolean {
  const severity = filter.severity.trim().toLowerCase();
  const kind = filter.kind.trim().toLowerCase();
  const resourceId = filter.resource_id.trim().toLowerCase();
  const windowMinutes = Number.parseInt(filter.window_minutes, 10);

  if (severity && String(event.severity ?? "").toLowerCase() !== severity) {
    return false;
  }

  if (kind && !String(event.kind ?? "").toLowerCase().includes(kind)) {
    return false;
  }

  if (!matchesEventResource(event, resourceId)) {
    return false;
  }

  if (!matchesEventWindow(event, windowMinutes, nowMs)) {
    return false;
  }

  return true;
}

// Returns the display stardate used in the top HUD strip.
export function stardateValue(now: Date): string {
  const yearStart = new Date(now.getFullYear(), 0, 1);
  const dayOfYear = Math.floor((now.getTime() - yearStart.getTime()) / 86400000) + 1;
  return `${String(now.getFullYear()).slice(2)}.${String(dayOfYear).padStart(3, "0")}`;
}

// Keeps transitional variables referenced while roadmap pages are progressively implemented.
export function keepRoadmapReferences(..._args: unknown[]): void {
  return;
}