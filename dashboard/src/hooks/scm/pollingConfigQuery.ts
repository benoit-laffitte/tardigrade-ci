import { gql } from "@apollo/client";

export const POLLING_CONFIGS_QUERY = gql`
  query PollingConfigs {
    polling_configs {
      repository_url
      provider
      enabled
      interval_secs
      branches
      last_polled_at
      updated_at
    }
  }
`;
