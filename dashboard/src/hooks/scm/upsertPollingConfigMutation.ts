import { gql } from "@apollo/client";

export const UPSERT_POLLING_CONFIG_MUTATION = gql`
  mutation UpsertPollingConfig($repository_url: String!, $provider: String!, $enabled: Boolean!, $interval_secs: Int!, $branches: [String!]!) {
    upsert_scm_polling_config(
      repository_url: $repository_url,
      provider: $provider,
      enabled: $enabled,
      interval_secs: $interval_secs,
      branches: $branches
    )
  }
`;
