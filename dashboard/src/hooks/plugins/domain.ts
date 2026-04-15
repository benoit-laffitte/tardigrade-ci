import { useCallback, useState } from "react";

import type {
  AdminRole,
  ApiErrorPayload,
  PluginAdminInput,
  PluginAuthorizationCheckResponse,
  PluginInfo,
  PluginPolicyInput
} from "../dashboardTypes";
import { usePluginActions } from "./actions";

interface PluginDomainParams {
  adminRole: AdminRole;
  roleCapabilities: {
    can_run_operations: boolean;
    can_mutate_sensitive: boolean;
  };
  log: (message: string, kind?: string) => void;
  audit: (action: string, target: string) => void;
}

// Owns plugins/policy roadmap state and actions so plugin governance stays isolated.
export function usePluginDomain({
  adminRole,
  roleCapabilities,
  log,
  audit
}: Readonly<PluginDomainParams>) {
  const [pluginAdminForm, setPluginAdminForm] = useState<PluginAdminInput>({
    name: "",
    production_tagged_context: false
  });
  const [pluginAdminMessage, setPluginAdminMessage] = useState("");
  const [pluginInventory, setPluginInventory] = useState<PluginInfo[]>([]);
  const [pluginPolicyForm, setPluginPolicyForm] = useState<PluginPolicyInput>({
    context: "global",
    granted_capabilities: []
  });
  const [pluginPolicyMessage, setPluginPolicyMessage] = useState("");
  const [pluginAuthorizationResult, setPluginAuthorizationResult] =
    useState<PluginAuthorizationCheckResponse | null>(null);
  const [effectivePolicyContext, setEffectivePolicyContext] = useState("global");
  const [effectiveGrantedCapabilities, setEffectiveGrantedCapabilities] = useState<string[]>([]);

  // Reads one API error payload and extracts actionable message for operator feedback.
  const parseApiErrorMessage = useCallback(async (response: Response): Promise<string> => {
    try {
      const payload = (await response.json()) as ApiErrorPayload;
      return payload.message ?? `HTTP ${response.status}`;
    } catch {
      return `HTTP ${response.status}`;
    }
  }, []);

  const actions = usePluginActions({
    adminRole,
    roleCapabilities,
    pluginAdminForm,
    setPluginAdminMessage,
    setPluginInventory,
    pluginPolicyForm,
    setPluginPolicyForm,
    setPluginPolicyMessage,
    setPluginAuthorizationResult,
    setEffectivePolicyContext,
    setEffectiveGrantedCapabilities,
    parseApiErrorMessage,
    log,
    audit
  });

  return {
    pluginAdminForm,
    setPluginAdminForm,
    pluginAdminMessage,
    setPluginAdminMessage,
    pluginInventory,
    setPluginInventory,
    pluginPolicyForm,
    setPluginPolicyForm,
    pluginPolicyMessage,
    setPluginPolicyMessage,
    pluginAuthorizationResult,
    setPluginAuthorizationResult,
    effectivePolicyContext,
    setEffectivePolicyContext,
    effectiveGrantedCapabilities,
    setEffectiveGrantedCapabilities,
    ...actions
  };
}