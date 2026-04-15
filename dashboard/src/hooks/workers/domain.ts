import { useState } from "react";

import type { AdminRole, Worker, WorkerControlInput } from "../dashboardTypes";
import { useWorkerActions } from "./actions";

interface WorkerDomainParams {
  adminRole: AdminRole;
  roleCapabilities: {
    can_run_operations: boolean;
    can_mutate_sensitive: boolean;
  };
  setWorkersSnapshot: (workers: Worker[]) => void;
  log: (message: string, kind?: string) => void;
  audit: (action: string, target: string) => void;
  refreshAll: () => Promise<void>;
}

// Owns worker-control roadmap state and actions so worker operations stay isolated.
export function useWorkerDomain({
  adminRole,
  roleCapabilities,
  setWorkersSnapshot,
  log,
  audit,
  refreshAll
}: Readonly<WorkerDomainParams>) {
  const [workerControlForm, setWorkerControlForm] = useState<WorkerControlInput>({
    worker_id: "",
    build_id: "",
    completion_status: "success",
    completion_log_line: ""
  });
  const [workerControlMessage, setWorkerControlMessage] = useState("");
  const [lastClaimResult, setLastClaimResult] = useState<string>("");

  const actions = useWorkerActions({
    adminRole,
    roleCapabilities,
    workerControlForm,
    setWorkerControlForm,
    setWorkerControlMessage,
    setLastClaimResult,
    setWorkersSnapshot,
    log,
    audit,
    refreshAll
  });

  return {
    workerControlForm,
    setWorkerControlForm,
    workerControlMessage,
    setWorkerControlMessage,
    lastClaimResult,
    setLastClaimResult,
    ...actions
  };
}