import { useQuery } from "@apollo/client";
import { POLLING_CONFIGS_QUERY } from "./pollingConfigQuery";

export function usePollingConfigs() {
  const { data, loading, error, refetch } = useQuery(POLLING_CONFIGS_QUERY);
  return {
    pollingConfigs: data?.polling_configs ?? [],
    loading,
    error,
    refetch
  };
}
