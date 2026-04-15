import { useCallback } from "react";

import type {
  AdminRole,
  ListScmWebhookRejectionsResponse,
  RuntimeMetricsApiResponse,
  ScmPollingInput,
  ScmPollingTickSummary,
  ScmWebhookOpsFilter,
  ScmWebhookRejectionEntry,
  WebhookSecurityInput
} from "../dashboardTypes";
import { normalizeAllowlistInput, normalizeBranchesInput } from "../dashboardUtils";

interface ScmActionsParams {
  adminRole: AdminRole;
  roleCapabilities: {
    can_mutate_sensitive: boolean;
  };
  webhookForm: WebhookSecurityInput;
  setWebhookMessage: React.Dispatch<React.SetStateAction<string>>;
  knownWebhookConfigs: Set<string>;
  setKnownWebhookConfigs: React.Dispatch<React.SetStateAction<Set<string>>>;
  pollingForm: ScmPollingInput;
  setPollingMessage: React.Dispatch<React.SetStateAction<string>>;
  setPollingTickSummary: React.Dispatch<React.SetStateAction<ScmPollingTickSummary | null>>;
  knownPollingStates: Map<string, boolean>;
  setKnownPollingStates: React.Dispatch<React.SetStateAction<Map<string, boolean>>>;
  scmWebhookOpsFilter: ScmWebhookOpsFilter;
  setScmWebhookOpsMessage: React.Dispatch<React.SetStateAction<string>>;
  setScmWebhookMetrics: React.Dispatch<React.SetStateAction<RuntimeMetricsApiResponse | null>>;
  setScmWebhookRejections: React.Dispatch<React.SetStateAction<ScmWebhookRejectionEntry[]>>;
  parseApiErrorMessage: (response: Response) => Promise<string>;
  log: (message: string, kind?: string) => void;
  audit: (action: string, target: string) => void;
  refreshAll: () => Promise<void>;
}

// Groups SCM security, polling, and webhook-ops callbacks in one dedicated domain hook.
export function useScmActions({
  adminRole,
  roleCapabilities,
  webhookForm,
  setWebhookMessage,
  knownWebhookConfigs,
  setKnownWebhookConfigs,
  pollingForm,
  setPollingMessage,
  setPollingTickSummary,
  knownPollingStates,
  setKnownPollingStates,
  scmWebhookOpsFilter,
  setScmWebhookOpsMessage,
  setScmWebhookMetrics,
  setScmWebhookRejections,
  parseApiErrorMessage,
  log,
  audit,
  refreshAll
}: Readonly<ScmActionsParams>) {
  // Saves webhook security settings for one repository/provider pair.
  const saveWebhookSecurityConfig = useCallback(
    async (event: { preventDefault: () => void }) => {
      event.preventDefault();
      if (!roleCapabilities.can_mutate_sensitive) {
        setWebhookMessage("Role insuffisant pour modifier la securite webhook.");
        log(`Role ${adminRole} ne peut pas modifier webhook security`, "warn");
        audit("webhook_security_update_denied", webhookForm.repository_url || "unknown");
        return;
      }

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
          audit("webhook_security_update", repository);
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
    [
      adminRole,
      audit,
      knownWebhookConfigs,
      log,
      roleCapabilities.can_mutate_sensitive,
      setKnownWebhookConfigs,
      setWebhookMessage,
      webhookForm
    ]
  );

  // Saves one SCM polling configuration for repository/provider pair.
  const saveScmPollingConfig = useCallback(
    async (event: { preventDefault: () => void }) => {
      event.preventDefault();
      if (!roleCapabilities.can_mutate_sensitive) {
        setPollingMessage("Role insuffisant pour modifier la configuration polling.");
        log(`Role ${adminRole} ne peut pas modifier polling config`, "warn");
        audit("polling_config_update_denied", pollingForm.repository_url || "unknown");
        return;
      }

      const repository = pollingForm.repository_url.trim();
      const configKey = `${repository.toLowerCase()}::${pollingForm.provider}`;
      const interval = Number.parseInt(pollingForm.interval_secs_text, 10);

      if (!repository || !Number.isFinite(interval) || interval <= 0) {
        setPollingMessage("Parametres invalides: repository requis et interval > 0.");
        log("Configuration polling invalide", "warn");
        return;
      }

      if (knownPollingStates.get(configKey) === true && !pollingForm.enabled) {
        const confirmed = globalThis.confirm(
          "Confirmer la desactivation de ce polling repository/provider ?"
        );
        if (!confirmed) {
          setPollingMessage("Desactivation annulee.");
          return;
        }
      }

      const payload = {
        repository_url: repository,
        provider: pollingForm.provider,
        enabled: pollingForm.enabled,
        interval_secs: interval,
        branches: normalizeBranchesInput(pollingForm.branches_text)
      };

      try {
        const response = await fetch("/scm/polling/configs", {
          method: "POST",
          headers: {
            "content-type": "application/json"
          },
          body: JSON.stringify(payload)
        });

        if (response.status === 204) {
          setKnownPollingStates((previous) => {
            const next = new Map(previous);
            next.set(configKey, pollingForm.enabled);
            return next;
          });
          setPollingMessage(
            pollingForm.enabled ? "Configuration polling enregistree." : "Polling desactive."
          );
          log(`Polling config sauvegardee pour ${repository} (${pollingForm.provider})`, "ok");
          audit("polling_config_update", repository);
          return;
        }

        if (response.status === 400) {
          setPollingMessage("Configuration polling invalide.");
          log("Rejet configuration polling (400)", "warn");
          return;
        }

        setPollingMessage("Erreur interne lors de la sauvegarde polling.");
        log(`Configuration polling en echec: HTTP ${response.status}`, "error");
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        setPollingMessage("Erreur reseau lors de la sauvegarde polling.");
        log(`Configuration polling en echec: ${message}`, "error");
      }
    },
    [
      adminRole,
      audit,
      knownPollingStates,
      log,
      pollingForm,
      roleCapabilities.can_mutate_sensitive,
      setKnownPollingStates,
      setPollingMessage
    ]
  );

  // Triggers one manual SCM polling tick and renders immediate summary.
  const runManualScmPollingTick = useCallback(async () => {
    try {
      const response = await fetch("/scm/polling/tick", {
        method: "POST"
      });

      if (!response.ok) {
        setPollingMessage(`Tick polling en echec (HTTP ${response.status}).`);
        log(`Tick polling en echec: HTTP ${response.status}`, "error");
        return;
      }

      const payload = (await response.json()) as ScmPollingTickSummary;
      setPollingTickSummary(payload);
      setPollingMessage(
        `Tick execute: ${payload.polled_repositories} repo(s), ${payload.enqueued_builds} build(s) enqueued.`
      );
      log(
        `Tick polling: repos=${payload.polled_repositories}, enqueued=${payload.enqueued_builds}`,
        "ok"
      );
      await refreshAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPollingMessage("Erreur reseau lors du tick polling.");
      log(`Tick polling en echec: ${message}`, "error");
    }
  }, [log, refreshAll, setPollingMessage, setPollingTickSummary]);

  // Refreshes SCM webhook counters and recent rejection diagnostics timeline.
  const refreshScmWebhookOperations = useCallback(async () => {
    const provider = scmWebhookOpsFilter.provider.trim();
    const repositoryUrl = scmWebhookOpsFilter.repository_url.trim();

    const query = new URLSearchParams();
    if (provider) {
      query.set("provider", provider);
    }
    if (repositoryUrl) {
      query.set("repository_url", repositoryUrl);
    }
    query.set("limit", "20");

    try {
      const [metricsResponse, diagnosticsResponse] = await Promise.all([
        fetch("/metrics", { method: "GET" }),
        fetch(`/scm/webhook-security/rejections?${query.toString()}`, { method: "GET" })
      ]);

      if (!metricsResponse.ok) {
        const details = await parseApiErrorMessage(metricsResponse);
        setScmWebhookOpsMessage(`Chargement metrics webhook en echec: ${details}.`);
        log(`Metrics webhook en echec: ${details}`, "error");
        return;
      }

      if (!diagnosticsResponse.ok) {
        const details = await parseApiErrorMessage(diagnosticsResponse);
        setScmWebhookOpsMessage(`Chargement diagnostics webhook en echec: ${details}.`);
        log(`Diagnostics webhook en echec: ${details}`, "error");
        return;
      }

      const metricsPayload = (await metricsResponse.json()) as RuntimeMetricsApiResponse;
      const diagnosticsPayload = (await diagnosticsResponse.json()) as ListScmWebhookRejectionsResponse;
      setScmWebhookMetrics(metricsPayload);
      setScmWebhookRejections(diagnosticsPayload.rejections);
      setScmWebhookOpsMessage(
        `Diagnostics webhook rafraichis (${diagnosticsPayload.rejections.length} rejection(s)).`
      );
      log(
        `Diagnostics webhook rafraichis: received=${metricsPayload.scm_webhook_received_total}, rejected=${metricsPayload.scm_webhook_rejected_total}`,
        "ok"
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setScmWebhookOpsMessage("Erreur reseau lors du chargement webhook operations.");
      log(`Webhook operations en echec: ${message}`, "error");
    }
  }, [
    log,
    parseApiErrorMessage,
    scmWebhookOpsFilter.provider,
    scmWebhookOpsFilter.repository_url,
    setScmWebhookMetrics,
    setScmWebhookOpsMessage,
    setScmWebhookRejections
  ]);

  return {
    saveWebhookSecurityConfig,
    saveScmPollingConfig,
    runManualScmPollingTick,
    refreshScmWebhookOperations
  };
}