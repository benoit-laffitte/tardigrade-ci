import type { ApolloClient } from "@apollo/client";
import { useCallback } from "react";

import {
  RUN_SCM_POLLING_TICK_MUTATION,
  SCM_WEBHOOK_OPERATIONS_QUERY,
  UPSERT_SCM_POLLING_CONFIG_MUTATION,
  UPSERT_WEBHOOK_SECURITY_CONFIG_MUTATION
} from "../dashboardConstants";

import type {
  AdminRole,
  RuntimeMetricsApiResponse,
  ScmPollingInput,
  ScmPollingTickSummary,
  ScmWebhookOpsFilter,
  ScmWebhookRejectionEntry,
  ScmWebhookOperationsResponse,
  RunScmPollingTickResponse,
  UpsertScmPollingConfigResponse,
  UpsertWebhookSecurityConfigResponse,
  WebhookSecurityInput
} from "../dashboardTypes";
import { normalizeAllowlistInput, normalizeBranchesInput } from "../dashboardUtils";

// Maps dashboard SCM provider ids to the GraphQL enum literals expected by async-graphql.
function toGraphQlScmProvider(provider: "github" | "gitlab"): "Github" | "Gitlab" {
  return provider === "github" ? "Github" : "Gitlab";
}

interface ScmActionsParams {
  client: ApolloClient<object>;
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
  log: (message: string, kind?: string) => void;
  audit: (action: string, target: string) => void;
  refreshAll: () => Promise<void>;
}

// Groups SCM security, polling, and webhook-ops callbacks in one dedicated domain hook.
export function useScmActions({
  client,
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
        provider: toGraphQlScmProvider(webhookForm.provider),
        secret,
        allowed_ips: normalizeAllowlistInput(webhookForm.allowed_ips_text)
      };

      try {
        const { data } = await client.mutate<UpsertWebhookSecurityConfigResponse>({
          mutation: UPSERT_WEBHOOK_SECURITY_CONFIG_MUTATION,
          variables: {
            input: payload
          }
        });

        if (!data?.upsert_webhook_security_config) {
          throw new Error("upsert_webhook_security_config did not acknowledge success");
        }

        setWebhookMessage("Configuration webhook enregistree.");
        setKnownWebhookConfigs((previous) => new Set(previous).add(configKey));
        log(`Webhook security sauvegardee pour ${repository} (${webhookForm.provider})`, "ok");
        audit("webhook_security_update", repository);
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        setWebhookMessage(`Erreur lors de la sauvegarde webhook: ${message}.`);
        log(`Configuration webhook en echec: ${message}`, "error");
      }
    },
    [
      adminRole,
      audit,
      client,
      knownWebhookConfigs,
      log,
      roleCapabilities.can_mutate_sensitive,
      setKnownWebhookConfigs,
      setWebhookMessage,
      webhookForm
    ]
  );

  // Saves SCM polling settings through the GraphQL control plane.
  const saveScmPollingConfig = useCallback(
    async (event: { preventDefault: () => void }) => {
      event.preventDefault();
      if (!roleCapabilities.can_mutate_sensitive) {
        setPollingMessage("Role insuffisant pour modifier le polling SCM.");
        log(`Role ${adminRole} ne peut pas modifier polling SCM`, "warn");
        audit("polling_config_update_denied", pollingForm.repository_url || "unknown");
        return;
      }

      const repository = pollingForm.repository_url.trim();
      const interval = Number.parseInt(pollingForm.interval_secs_text, 10);
      const configKey = `${repository.toLowerCase()}::${pollingForm.provider}`;

      if (!repository || Number.isNaN(interval) || interval <= 0) {
        setPollingMessage("Parametres invalides pour le polling.");
        log("Configuration polling invalide", "warn");
        return;
      }

      const knownState = knownPollingStates.get(configKey);
      if (knownState !== undefined && knownState !== pollingForm.enabled) {
        const confirmed = globalThis.confirm(
          "Une configuration de polling existe deja pour ce repository/provider. Confirmer le changement ?"
        );
        if (!confirmed) {
          setPollingMessage("Modification du polling annulee.");
          return;
        }
      }

      const payload = {
        repository_url: repository,
        provider: toGraphQlScmProvider(pollingForm.provider),
        enabled: pollingForm.enabled,
        interval_secs: interval,
        branches: normalizeBranchesInput(pollingForm.branches_text)
      };

      try {
        const { data } = await client.mutate<UpsertScmPollingConfigResponse>({
          mutation: UPSERT_SCM_POLLING_CONFIG_MUTATION,
          variables: {
            input: payload
          }
        });

        if (!data?.upsert_scm_polling_config) {
          throw new Error("upsert_scm_polling_config did not acknowledge success");
        }

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
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        setPollingMessage(`Erreur lors de la sauvegarde polling: ${message}.`);
        log(`Configuration polling en echec: ${message}`, "error");
      }
    },
    [
      adminRole,
      audit,
      client,
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
      const { data } = await client.mutate<RunScmPollingTickResponse>({
        mutation: RUN_SCM_POLLING_TICK_MUTATION
      });

      if (!data?.run_scm_polling_tick) {
        throw new Error("run_scm_polling_tick did not return a summary");
      }

      const payload = data.run_scm_polling_tick;
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
      setPollingMessage(`Erreur lors du tick polling: ${message}.`);
      log(`Tick polling en echec: ${message}`, "error");
    }
  }, [client, log, refreshAll, setPollingMessage, setPollingTickSummary]);

  // Refreshes SCM webhook counters and recent rejection diagnostics timeline.
  const refreshScmWebhookOperations = useCallback(async () => {
    const provider = scmWebhookOpsFilter.provider.trim();
    const repositoryUrl = scmWebhookOpsFilter.repository_url.trim();

    try {
      const { data } = await client.query<ScmWebhookOperationsResponse>({
        query: SCM_WEBHOOK_OPERATIONS_QUERY,
        variables: {
          provider: provider || null,
          repositoryUrl: repositoryUrl || null,
          limit: 20
        },
        fetchPolicy: "network-only"
      });

      if (!data?.metrics) {
        throw new Error("scm webhook diagnostics query did not return metrics");
      }

      setScmWebhookMetrics(data.metrics);
      setScmWebhookRejections(data.scm_webhook_rejections);
      setScmWebhookOpsMessage(
        `Diagnostics webhook rafraichis (${data.scm_webhook_rejections.length} rejection(s)).`
      );
      log(
        `Diagnostics webhook rafraichis: received=${data.metrics.scm_webhook_received_total}, rejected=${data.metrics.scm_webhook_rejected_total}`,
        "ok"
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setScmWebhookOpsMessage("Erreur reseau lors du chargement webhook operations.");
      log(`Webhook operations en echec: ${message}`, "error");
    }
  }, [
    client,
    log,
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