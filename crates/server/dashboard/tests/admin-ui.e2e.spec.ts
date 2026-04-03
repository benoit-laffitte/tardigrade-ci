import { expect, test } from "@playwright/test";

/**
 * Mocks baseline dashboard API calls used during app bootstrap.
 */
async function mockDashboardBootstrap(page: Parameters<typeof test>[0]["page"]) {
  await page.route("**/graphql", async (route) => {
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
              jobs: [],
              builds: [],
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

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ data: {} })
    });
  });

  await page.route("**/plugins", async (route) => {
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

  await page.route("**/plugins/policies**", async (route) => {
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

  await page.route("**/plugins/**/authorize-check", async (route) => {
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

  await page.route("**/metrics", async (route) => {
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

  await page.route("**/scm/webhook-security/rejections**", async (route) => {
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

  await page.route("**/workers", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ workers: [] })
    });
  });

  await page.route("**/scm/polling/tick", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ polled_repositories: 1, enqueued_builds: 0 })
    });
  });

  await page.route("**/scm/polling/configs", async (route) => {
    await route.fulfill({ status: 204, body: "" });
  });

  await page.route("**/scm/webhook-security/configs", async (route) => {
    await route.fulfill({ status: 500, body: "" });
  });
}

/**
 * Installs EventSource shim so tests do not rely on real SSE server.
 */
async function mockEventSource(page: Parameters<typeof test>[0]["page"]) {
  await page.addInitScript(() => {
    class MockEventSource {
      onopen: ((this: EventSource, ev: Event) => unknown) | null = null;
      onerror: ((this: EventSource, ev: Event) => unknown) | null = null;
      onmessage: ((this: EventSource, ev: MessageEvent<string>) => unknown) | null = null;

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
    window.EventSource = MockEventSource;
  });
}

test.beforeEach(async ({ page }) => {
  await mockEventSource(page);
  await mockDashboardBootstrap(page);
});

test("viewer role blocks sensitive plugin action", async ({ page }) => {
  await page.goto("/");

  await page.selectOption("select[name='admin_role']", "viewer");
  await page.fill("input[name='plugin_admin_name']", "net-diagnostics");
  await page.click("button:has-text('Load')");

  await expect(page.getByText("Role insuffisant pour charger un plugin.")).toBeVisible();
  await expect(page.getByText("plugin_load_denied")).toBeVisible();
});

test("plugin policy dry-run shows deny with missing capability", async ({ page }) => {
  await page.goto("/");

  await page.fill("input[name='plugin_admin_name']", "net-diagnostics");
  await page.click("button:has-text('Dry-run authorize')");

  await expect(page.getByText("Policy deny pour net-diagnostics")).toBeVisible();
  await expect(page.getByText("Deny: missing=network.")).toBeVisible();
});

test("webhook operations panel shows counters and rejection timeline", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("Webhook Security Operations")).toBeVisible();
  await expect(page.getByText("Received")).toBeVisible();
  await expect(page.getByText("invalid_webhook_signature")).toBeVisible();
});

test("observability panel filters event by resource id", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("Advanced Observability")).toBeVisible();
  await page.fill("input[name='observability_resource']", "worker-123");
  await expect(page.getByText("job=job-123 | build=build-123 | worker=worker-123")).toBeVisible();
});

test("webhook security save surfaces backend error", async ({ page }) => {
  await page.goto("/");

  await page.fill("input[name='webhook_repository_url']", "https://example.com/repo.git");
  await page.fill("input[name='webhook_secret']", "super-secret");
  await page.click("button:has-text('Enregistrer')");

  await expect(page.getByText("Erreur interne lors de la sauvegarde webhook.")).toBeVisible();
});
