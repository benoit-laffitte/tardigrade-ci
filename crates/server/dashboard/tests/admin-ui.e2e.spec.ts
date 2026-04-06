import { expect, test, type Page, type Route } from "@playwright/test";

/**
 * Mocks dashboard API calls used by the API-backed pages (Pipelines/Overview).
 */
async function mockDashboardBootstrap(page: Page) {
  const jobs = [
    {
      id: "job-12345678",
      name: "build-api",
      repository_url: "https://example.com/repo.git",
      pipeline_path: "pipelines/api.yml",
      created_at: new Date().toISOString()
    }
  ];

  const builds = [
    {
      id: "build-running-1",
      job_id: "job-12345678",
      status: "Running",
      queued_at: new Date().toISOString(),
      started_at: new Date().toISOString(),
      finished_at: null,
      logs: []
    },
    {
      id: "build-success-1",
      job_id: "job-12345678",
      status: "Success",
      queued_at: new Date().toISOString(),
      started_at: new Date().toISOString(),
      finished_at: new Date().toISOString(),
      logs: []
    },
    {
      id: "build-failed-1",
      job_id: "job-12345678",
      status: "Failed",
      queued_at: new Date().toISOString(),
      started_at: new Date().toISOString(),
      finished_at: new Date().toISOString(),
      logs: []
    }
  ];

  await page.route("**/graphql", async (route: Route) => {
    const request = route.request();
    const body = request.postDataJSON() as { query?: string };
    const query = body.query ?? "";

    if (query.includes("DashboardSnapshot")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          data: {
            dashboard_snapshot: {
              jobs,
              builds,
              workers: [],
              metrics: {
                reclaimed_total: 0,
                retry_requeued_total: 0,
                ownership_conflicts_total: 0,
                dead_letter_total: 0
              },
              dead_letter_builds: []
            }
          }
        })
      });
      return;
    }

    if (query.includes("CreateJob")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          data: {
            create_job: {
              id: "job-created-1",
              name: "build-web"
            }
          }
        })
      });
      return;
    }

    if (query.includes("RunJob")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          data: {
            run_job: {
              id: "build-new-run-1"
            }
          }
        })
      });
      return;
    }

    if (query.includes("CancelBuild")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          data: {
            cancel_build: {
              id: "build-running-1"
            }
          }
        })
      });
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ data: {} })
    });
  });

  await page.route("**/health", async (route: Route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "ok" })
    });
  });

  await page.route("**/plugins", async (route: Route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ plugins: [] })
      });
      return;
    }

    await route.fulfill({
      status: 201,
      contentType: "application/json",
      body: JSON.stringify({
        status: "loaded",
        plugin: {
          name: "net-diagnostics",
          state: "Loaded",
          capabilities: ["network"],
          source_manifest_entry: "builtin:net-diagnostics"
        }
      })
    });
  });

  await page.route("**/plugins/policies**", async (route: Route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          context: "global",
          granted_capabilities: ["filesystem"]
        })
      });
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        context: "global",
        granted_capabilities: ["filesystem"]
      })
    });
  });

  await page.route("**/plugins/**/authorize-check", async (route: Route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        plugin_name: "net-diagnostics",
        context: "global",
        required_capabilities: ["network"],
        granted_capabilities: ["filesystem"],
        missing_capabilities: ["network"],
        allowed: false
      })
    });
  });

  await page.route("**/metrics", async (route: Route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        reclaimed_total: 0,
        retry_requeued_total: 0,
        ownership_conflicts_total: 0,
        dead_letter_total: 0,
        scm_webhook_received_total: 5,
        scm_webhook_accepted_total: 3,
        scm_webhook_rejected_total: 2,
        scm_webhook_duplicate_total: 1,
        scm_trigger_enqueued_builds_total: 0,
        scm_polling_ticks_total: 0,
        scm_polling_repositories_total: 0,
        scm_polling_enqueued_builds_total: 0
      })
    });
  });

  await page.route("**/scm/webhook-security/rejections**", async (route: Route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        rejections: [
          {
            reason_code: "invalid_webhook_signature",
            provider: "github",
            repository_url: "https://example.com/repo.git",
            at: new Date().toISOString()
          }
        ]
      })
    });
  });

  await page.route("**/workers", async (route: Route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ workers: [] })
    });
  });

  await page.route("**/scm/polling/tick", async (route: Route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ polled_repositories: 1, enqueued_builds: 0 })
    });
  });

  await page.route("**/scm/polling/configs", async (route: Route) => {
    await route.fulfill({ status: 204, body: "" });
  });

  await page.route("**/scm/webhook-security/configs", async (route: Route) => {
    await route.fulfill({ status: 500, body: "" });
  });
}

/**
 * Installs EventSource shim so tests do not rely on real SSE server.
 */
async function mockEventSource(page: Page) {
  await page.addInitScript(() => {
    class MockEventSource {
      onopen: ((ev: Event) => unknown) | null = null;
      onerror: ((ev: Event) => unknown) | null = null;
      onmessage: ((ev: MessageEvent<string>) => unknown) | null = null;

      close() {
        // no-op for tests
      }

      constructor() {
        setTimeout(() => {
          this.onopen?.(new Event("open"));
          this.onmessage?.(
            new MessageEvent("message", {
              data: JSON.stringify({
                kind: "scm_webhook_ingested",
                severity: "info",
                message: "event mocked",
                at: new Date().toISOString(),
                job_id: "job-123",
                build_id: "build-123",
                worker_id: "worker-123"
              })
            })
          );
        }, 20);
      }
    }

    // @ts-expect-error test shim
    globalThis.EventSource = MockEventSource;
  });
}

test.beforeEach(async ({ page }) => {
  await mockEventSource(page);
  await mockDashboardBootstrap(page);
});

/**
 * Verifies that all UX target pages are present in the navigation shell.
 */
test("shows 7-page navigation shell", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("button", { name: "Pipelines" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Overview" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Workers" })).toBeVisible();
  await expect(page.getByRole("button", { name: "SCM Security" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Plugins & Policy" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Observability" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Administration" })).toBeVisible();
});

/**
 * Verifies API coverage gating and roadmap placeholder behavior on non-implemented pages.
 */
test("shows roadmap gating on non-implemented page", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "Workers" }).click();

  await expect(page.getByText("Perimetre API reel")).toBeVisible();
  await expect(page.locator(".api-coverage-panel .pill")).toHaveText("roadmap");
  await expect(page.getByRole("heading", { name: "Page en mode roadmap" })).toBeVisible();
});

/**
 * Verifies that Pipelines page exposes API-backed actions and endpoint-labeled controls.
 */
test("pipelines page shows API-backed actions", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "Pipelines" }).click();

  await expect(page.locator(".api-coverage-panel .pill")).toHaveText("full");
  await expect(page.getByRole("button", { name: "POST /jobs", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "POST /jobs/{id}/run" })).toBeVisible();
  await expect(page.locator("button:has-text('POST /builds/{id}/cancel'):not([disabled])")).toHaveCount(1);
});

/**
 * Verifies create-job flow wired through API-backed Pipelines page.
 */
test("creates job from pipelines page", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "Pipelines" }).click();

  await page.fill("input[name='name']", "build-web");
  await page.fill("input[name='repository_url']", "https://example.com/web.git");
  await page.fill("input[name='pipeline_path']", "pipelines/web.yml");
  await page.getByRole("button", { name: "POST /jobs", exact: true }).click();

  await expect(page.getByText("Job build-web cree.")).toBeVisible();
});

/**
 * Verifies Overview page stays API-strict and shows derived build/job/health summary.
 */
test("overview page displays API-strict summary", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "Overview" }).click();

  await expect(page.locator(".api-coverage-panel .pill")).toHaveText("partial");
  await expect(page.getByText("Health & Delivery Snapshot")).toBeVisible();
  await expect(page.getByText("Build Status Breakdown")).toBeVisible();
  await expect(page.getByText("API-backed Overview Scope")).toBeVisible();
  await expect(page.getByText("No reliance on /metrics, /events, /dead-letter-builds for this page.")).toBeVisible();
});
