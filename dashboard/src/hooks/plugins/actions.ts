import { useCallback } from "react";

import type {
  AdminRole,
  ListPluginsResponse,
  PluginActionResponse,
  PluginAdminInput,
  PluginAuthorizationCheckResponse,
  PluginInfo,
  PluginPolicyInput,
  PluginPolicyResponse
} from "../dashboardTypes";

interface PluginActionsParams {
  adminRole: AdminRole;
  roleCapabilities: {
    can_run_operations: boolean;
    can_mutate_sensitive: boolean;
  };
  pluginAdminForm: PluginAdminInput;
  setPluginAdminMessage: React.Dispatch<React.SetStateAction<string>>;
  setPluginInventory: React.Dispatch<React.SetStateAction<PluginInfo[]>>;
  pluginPolicyForm: PluginPolicyInput;
  setPluginPolicyForm: React.Dispatch<React.SetStateAction<PluginPolicyInput>>;
  setPluginPolicyMessage: React.Dispatch<React.SetStateAction<string>>;
  setPluginAuthorizationResult: React.Dispatch<React.SetStateAction<PluginAuthorizationCheckResponse | null>>;
  setEffectivePolicyContext: React.Dispatch<React.SetStateAction<string>>;
  setEffectiveGrantedCapabilities: React.Dispatch<React.SetStateAction<string[]>>;
  parseApiErrorMessage: (response: Response) => Promise<string>;
  log: (message: string, kind?: string) => void;
  audit: (action: string, target: string) => void;
}

// Groups plugin lifecycle and policy callbacks in one dedicated domain hook.
export function usePluginActions({
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
}: Readonly<PluginActionsParams>) {
  // Refreshes plugin registry inventory for administration panel.
  const refreshPluginInventory = useCallback(async () => {
    try {
      const response = await fetch("/plugins", { method: "GET" });
      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Chargement plugins en echec: ${details}.`);
        log(`Chargement plugins en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as ListPluginsResponse;
      setPluginInventory(payload.plugins);
      setPluginAdminMessage(`Inventaire plugins rafraichi (${payload.plugins.length}).`);
      log(`Inventaire plugins rafraichi (${payload.plugins.length})`, "ok");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors du chargement plugins.");
      log(`Chargement plugins en echec: ${message}`, "error");
    }
  }, [log, parseApiErrorMessage, setPluginAdminMessage, setPluginInventory]);

  // Loads one plugin from built-in server catalog.
  const loadPlugin = useCallback(async () => {
    if (!roleCapabilities.can_mutate_sensitive) {
      setPluginAdminMessage("Role insuffisant pour charger un plugin.");
      log(`Role ${adminRole} ne peut pas charger de plugin`, "warn");
      audit("plugin_load_denied", pluginAdminForm.name || "unknown");
      return;
    }

    const name = pluginAdminForm.name.trim();
    if (!name) {
      setPluginAdminMessage("Nom plugin requis.");
      log("Load plugin refuse: nom manquant", "warn");
      return;
    }

    try {
      const response = await fetch("/plugins", {
        method: "POST",
        headers: {
          "content-type": "application/json"
        },
        body: JSON.stringify({ name })
      });

      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Load plugin en echec: ${details}.`);
        log(`Load plugin ${name} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginActionResponse;
      setPluginAdminMessage(`Plugin ${payload.plugin.name} ${payload.status}.`);
      log(`Plugin ${payload.plugin.name} ${payload.status}`, "ok");
      audit("plugin_load", payload.plugin.name);
      await refreshPluginInventory();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors du chargement plugin.");
      log(`Load plugin ${name} en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginAdminForm.name,
    refreshPluginInventory,
    roleCapabilities.can_mutate_sensitive,
    setPluginAdminMessage
  ]);

  // Initializes one already loaded plugin.
  const initPlugin = useCallback(async () => {
    if (!roleCapabilities.can_run_operations) {
      setPluginAdminMessage("Role insuffisant pour initialiser un plugin.");
      log(`Role ${adminRole} ne peut pas initialiser de plugin`, "warn");
      audit("plugin_init_denied", pluginAdminForm.name || "unknown");
      return;
    }

    const name = pluginAdminForm.name.trim();
    if (!name) {
      setPluginAdminMessage("Nom plugin requis.");
      log("Init plugin refuse: nom manquant", "warn");
      return;
    }

    try {
      const response = await fetch(`/plugins/${encodeURIComponent(name)}/init`, {
        method: "POST"
      });
      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Init plugin en echec: ${details}.`);
        log(`Init plugin ${name} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginActionResponse;
      setPluginAdminMessage(`Plugin ${payload.plugin.name} ${payload.status}.`);
      log(`Plugin ${payload.plugin.name} ${payload.status}`, "ok");
      audit("plugin_init", payload.plugin.name);
      await refreshPluginInventory();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors de l'initialisation plugin.");
      log(`Init plugin ${name} en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginAdminForm.name,
    refreshPluginInventory,
    roleCapabilities.can_run_operations,
    setPluginAdminMessage
  ]);

  // Executes one plugin, requiring confirmation when context is production tagged.
  const executePlugin = useCallback(async () => {
    if (!roleCapabilities.can_run_operations) {
      setPluginAdminMessage("Role insuffisant pour executer un plugin.");
      log(`Role ${adminRole} ne peut pas executer de plugin`, "warn");
      audit("plugin_execute_denied", pluginAdminForm.name || "unknown");
      return;
    }

    const name = pluginAdminForm.name.trim();
    if (!name) {
      setPluginAdminMessage("Nom plugin requis.");
      log("Execute plugin refuse: nom manquant", "warn");
      return;
    }

    if (pluginAdminForm.production_tagged_context) {
      const confirmed = globalThis.confirm(
        "Contexte tagge production: confirmer l'execution diagnostique du plugin ?"
      );
      if (!confirmed) {
        setPluginAdminMessage("Execution plugin annulee.");
        return;
      }
    }

    try {
      const response = await fetch(`/plugins/${encodeURIComponent(name)}/execute`, {
        method: "POST"
      });
      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Execute plugin en echec: ${details}.`);
        log(`Execute plugin ${name} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginActionResponse;
      setPluginAdminMessage(`Plugin ${payload.plugin.name} ${payload.status}.`);
      log(`Plugin ${payload.plugin.name} ${payload.status}`, "ok");
      audit("plugin_execute", payload.plugin.name);
      await refreshPluginInventory();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors de l'execution plugin.");
      log(`Execute plugin ${name} en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginAdminForm,
    refreshPluginInventory,
    roleCapabilities.can_run_operations,
    setPluginAdminMessage
  ]);

  // Unloads one plugin after explicit operator confirmation.
  const unloadPlugin = useCallback(async () => {
    if (!roleCapabilities.can_mutate_sensitive) {
      setPluginAdminMessage("Role insuffisant pour decharger un plugin.");
      log(`Role ${adminRole} ne peut pas decharger de plugin`, "warn");
      audit("plugin_unload_denied", pluginAdminForm.name || "unknown");
      return;
    }

    const name = pluginAdminForm.name.trim();
    if (!name) {
      setPluginAdminMessage("Nom plugin requis.");
      log("Unload plugin refuse: nom manquant", "warn");
      return;
    }

    const confirmed = globalThis.confirm(`Confirmer le dechargement du plugin ${name} ?`);
    if (!confirmed) {
      setPluginAdminMessage("Dechargement plugin annule.");
      return;
    }

    try {
      const response = await fetch(`/plugins/${encodeURIComponent(name)}/unload`, {
        method: "POST"
      });
      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginAdminMessage(`Unload plugin en echec: ${details}.`);
        log(`Unload plugin ${name} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginActionResponse;
      setPluginAdminMessage(`Plugin ${payload.plugin.name} ${payload.status}.`);
      log(`Plugin ${payload.plugin.name} ${payload.status}`, "ok");
      audit("plugin_unload", payload.plugin.name);
      await refreshPluginInventory();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginAdminMessage("Erreur reseau lors du dechargement plugin.");
      log(`Unload plugin ${name} en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginAdminForm.name,
    refreshPluginInventory,
    roleCapabilities.can_mutate_sensitive,
    setPluginAdminMessage
  ]);

  // Toggles one capability in plugin policy form while preserving uniqueness.
  const togglePluginPolicyCapability = useCallback(
    (capability: string, checked: boolean) => {
      setPluginPolicyForm((previous) => {
        const nextCapabilities = checked
          ? Array.from(new Set([...previous.granted_capabilities, capability]))
          : previous.granted_capabilities.filter((value) => value !== capability);
        return { ...previous, granted_capabilities: nextCapabilities };
      });
    },
    [setPluginPolicyForm]
  );

  // Loads effective policy values for selected context and syncs form toggles.
  const loadPluginPolicy = useCallback(async () => {
    const context = pluginPolicyForm.context.trim() || "global";
    try {
      const response = await fetch(`/plugins/policies?context=${encodeURIComponent(context)}`, {
        method: "GET"
      });

      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginPolicyMessage(`Chargement policy en echec: ${details}.`);
        log(`Chargement policy plugin en echec (${context}): ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginPolicyResponse;
      setEffectivePolicyContext(payload.context);
      setEffectiveGrantedCapabilities(payload.granted_capabilities);
      setPluginPolicyForm((previous) => ({
        ...previous,
        context: payload.context,
        granted_capabilities: payload.granted_capabilities
      }));
      setPluginPolicyMessage(
        `Policy chargee (${payload.context}): ${payload.granted_capabilities.join(", ") || "none"}.`
      );
      log(
        `Policy plugin chargee (${payload.context}) caps=${payload.granted_capabilities.join(",") || "none"}`,
        "ok"
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginPolicyMessage("Erreur reseau lors du chargement policy.");
      log(`Chargement policy plugin en echec (${context}): ${message}`, "error");
    }
  }, [
    log,
    parseApiErrorMessage,
    pluginPolicyForm.context,
    setEffectiveGrantedCapabilities,
    setEffectivePolicyContext,
    setPluginPolicyForm,
    setPluginPolicyMessage
  ]);

  // Saves granted capabilities for selected plugin execution context.
  const savePluginPolicy = useCallback(async () => {
    if (!roleCapabilities.can_mutate_sensitive) {
      setPluginPolicyMessage("Role insuffisant pour modifier la policy plugin.");
      log(`Role ${adminRole} ne peut pas modifier plugin policy`, "warn");
      audit("plugin_policy_update_denied", pluginPolicyForm.context || "global");
      return;
    }

    const context = pluginPolicyForm.context.trim() || "global";
    const wantsSecrets = pluginPolicyForm.granted_capabilities.includes("secrets");

    if (wantsSecrets) {
      const confirmed = globalThis.confirm(
        "Confirmer l'octroi de la capacite secrets pour ce contexte ?"
      );
      if (!confirmed) {
        setPluginPolicyMessage("Mise a jour policy annulee.");
        return;
      }
    }

    try {
      const response = await fetch("/plugins/policies", {
        method: "POST",
        headers: {
          "content-type": "application/json"
        },
        body: JSON.stringify({
          context,
          granted_capabilities: pluginPolicyForm.granted_capabilities
        })
      });

      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginPolicyMessage(`Policy en echec: ${details}.`);
        log(`Policy plugin en echec (${context}): ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginPolicyResponse;
      setEffectivePolicyContext(payload.context);
      setEffectiveGrantedCapabilities(payload.granted_capabilities);
      setPluginPolicyMessage(
        `Policy enregistree (${payload.context}): ${payload.granted_capabilities.join(", ") || "none"}.`
      );
      audit("plugin_policy_update", payload.context);
      log(
        `Policy plugin sauvegardee (${payload.context}) caps=${payload.granted_capabilities.join(",") || "none"}`,
        "ok"
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginPolicyMessage("Erreur reseau lors de la sauvegarde policy.");
      log(`Policy plugin en echec: ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    parseApiErrorMessage,
    pluginPolicyForm,
    roleCapabilities.can_mutate_sensitive,
    setEffectiveGrantedCapabilities,
    setEffectivePolicyContext,
    setPluginPolicyMessage
  ]);

  // Runs authorization dry-run for selected plugin and context, then renders allow/deny diff.
  const runPluginAuthorizationCheck = useCallback(async () => {
    const pluginName = pluginAdminForm.name.trim();
    const context = pluginPolicyForm.context.trim() || "global";

    if (!pluginName) {
      setPluginPolicyMessage("Nom plugin requis pour verification policy.");
      log("Authorize-check refuse: nom plugin manquant", "warn");
      return;
    }

    try {
      const response = await fetch(`/plugins/${encodeURIComponent(pluginName)}/authorize-check`, {
        method: "POST",
        headers: {
          "content-type": "application/json"
        },
        body: JSON.stringify({ context })
      });

      if (!response.ok) {
        const details = await parseApiErrorMessage(response);
        setPluginPolicyMessage(`Authorize-check en echec: ${details}.`);
        log(`Authorize-check plugin ${pluginName} en echec: ${details}`, "error");
        return;
      }

      const payload = (await response.json()) as PluginAuthorizationCheckResponse;
      setPluginAuthorizationResult(payload);
      if (payload.allowed) {
        setPluginPolicyMessage(`Policy allow pour ${payload.plugin_name} (${payload.context}).`);
        log(`Policy allow ${payload.plugin_name} (${payload.context})`, "ok");
      } else {
        setPluginPolicyMessage(
          `Policy deny pour ${payload.plugin_name}: missing ${payload.missing_capabilities.join(", ")}.`
        );
        log(
          `Policy deny ${payload.plugin_name} (${payload.context}) missing=${payload.missing_capabilities.join(",")}`,
          "warn"
        );
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setPluginPolicyMessage("Erreur reseau lors du dry-run policy.");
      log(`Authorize-check plugin ${pluginName} en echec: ${message}`, "error");
    }
  }, [
    log,
    parseApiErrorMessage,
    pluginAdminForm.name,
    pluginPolicyForm.context,
    setPluginAuthorizationResult,
    setPluginPolicyMessage
  ]);

  return {
    refreshPluginInventory,
    loadPlugin,
    initPlugin,
    executePlugin,
    unloadPlugin,
    togglePluginPolicyCapability,
    loadPluginPolicy,
    savePluginPolicy,
    runPluginAuthorizationCheck
  };
}