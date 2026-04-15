import { useApolloClient } from "@apollo/client";
import { useState } from "react";

import type {
  AdminRole,
  RuntimeMetricsApiResponse,
  ScmPollingInput,
  ScmPollingTickSummary,
  ScmWebhookOpsFilter,
  ScmWebhookRejectionEntry,
  WebhookSecurityInput
} from "../dashboardTypes";
import { useScmActions } from "./actions";

interface ScmDomainParams {
  adminRole: AdminRole;
  roleCapabilities: {
    can_mutate_sensitive: boolean;
  };
  log: (message: string, kind?: string) => void;
  audit: (action: string, target: string) => void;
  refreshAll: () => Promise<void>;
}

// Owns SCM roadmap state and actions so SCM logic can evolve independently.
export function useScmDomain({
  adminRole,
  roleCapabilities,
  log,
  audit,
  refreshAll
}: Readonly<ScmDomainParams>) {
  const client = useApolloClient();
  const [webhookForm, setWebhookForm] = useState<WebhookSecurityInput>({
    repository_url: "",
    provider: "github",
    secret: "",
    allowed_ips_text: ""
  });
  const [webhookMessage, setWebhookMessage] = useState("");
  const [showWebhookSecret, setShowWebhookSecret] = useState(false);
  const [knownWebhookConfigs, setKnownWebhookConfigs] = useState<Set<string>>(new Set());
  const [pollingForm, setPollingForm] = useState<ScmPollingInput>({
    repository_url: "",
    provider: "github",
    enabled: true,
    interval_secs_text: "30",
    branches_text: "main"
  });
  const [pollingMessage, setPollingMessage] = useState("");
  const [pollingTickSummary, setPollingTickSummary] = useState<ScmPollingTickSummary | null>(null);
  const [knownPollingStates, setKnownPollingStates] = useState<Map<string, boolean>>(new Map());
  const [scmWebhookOpsFilter, setScmWebhookOpsFilter] = useState<ScmWebhookOpsFilter>({
    provider: "",
    repository_url: ""
  });
  const [scmWebhookOpsMessage, setScmWebhookOpsMessage] = useState("");
  const [scmWebhookMetrics, setScmWebhookMetrics] = useState<RuntimeMetricsApiResponse | null>(null);
  const [scmWebhookRejections, setScmWebhookRejections] = useState<ScmWebhookRejectionEntry[]>([]);

  const actions = useScmActions({
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
  });

  return {
    webhookForm,
    setWebhookForm,
    webhookMessage,
    setWebhookMessage,
    showWebhookSecret,
    setShowWebhookSecret,
    knownWebhookConfigs,
    setKnownWebhookConfigs,
    pollingForm,
    setPollingForm,
    pollingMessage,
    setPollingMessage,
    pollingTickSummary,
    setPollingTickSummary,
    knownPollingStates,
    setKnownPollingStates,
    scmWebhookOpsFilter,
    setScmWebhookOpsFilter,
    scmWebhookOpsMessage,
    setScmWebhookOpsMessage,
    scmWebhookMetrics,
    setScmWebhookMetrics,
    scmWebhookRejections,
    setScmWebhookRejections,
    ...actions
  };
}