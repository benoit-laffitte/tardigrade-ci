import { useQuery } from "@apollo/client";
import { WEBHOOK_SECURITY_CONFIG_QUERY } from "./webhookSecurityConfigQuery";

export function useWebhookSecurityConfig(repositoryUrl: string, provider: "Github" | "Gitlab") {
  const { data, loading, error } = useQuery(WEBHOOK_SECURITY_CONFIG_QUERY, {
    variables: { repositoryUrl, provider },
    skip: !repositoryUrl || !provider
  });
  return {
    config: data?.webhook_security_config ?? null,
    loading,
    error
  };
}
