import { useMutation } from "@apollo/client";
import { UPSERT_POLLING_CONFIG_MUTATION } from "./upsertPollingConfigMutation";

export function useUpsertPollingConfig() {
  const [upsertPollingConfig, { loading, error }] = useMutation(UPSERT_POLLING_CONFIG_MUTATION);
  return { upsertPollingConfig, loading, error };
}
