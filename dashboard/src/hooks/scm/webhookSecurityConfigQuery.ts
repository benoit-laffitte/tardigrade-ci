import { gql } from "@apollo/client";

export const WEBHOOK_SECURITY_CONFIG_QUERY = gql`
  query WebhookSecurityConfig($repositoryUrl: String!, $provider: GqlScmProvider!) {
    webhook_security_config(repository_url: $repositoryUrl, provider: $provider) {
      repository_url
      provider
      secret_masked
      allowed_ips
    }
  }
`;
