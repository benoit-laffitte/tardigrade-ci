import { useCallback } from "react";

import type {
  AdminRole,
  ClaimBuildResponse,
  CompleteBuildResponse,
  Worker,
  ListWorkersResponse,
  WorkerControlInput
} from "../dashboardTypes";

interface WorkerActionsParams {
  adminRole: AdminRole;
  roleCapabilities: {
    can_run_operations: boolean;
    can_mutate_sensitive: boolean;
  };
  workerControlForm: WorkerControlInput;
  setWorkerControlForm: React.Dispatch<React.SetStateAction<WorkerControlInput>>;
  setWorkerControlMessage: React.Dispatch<React.SetStateAction<string>>;
  setLastClaimResult: React.Dispatch<React.SetStateAction<string>>;
  setWorkersSnapshot: (workers: Worker[]) => void;
  log: (message: string, kind?: string) => void;
  audit: (action: string, target: string) => void;
  refreshAll: () => Promise<void>;
}

// Groups worker control callbacks in one dedicated domain hook.
export function useWorkerActions({
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
}: Readonly<WorkerActionsParams>) {
  // Fetches worker list from worker API and updates worker panel state.
  const refreshWorkers = useCallback(async () => {
    try {
      const response = await fetch("/workers", { method: "GET" });
      if (!response.ok) {
        setWorkerControlMessage(`Impossible de charger les workers (HTTP ${response.status}).`);
        log(`Chargement workers en echec: HTTP ${response.status}`, "error");
        return;
      }

      const payload = (await response.json()) as ListWorkersResponse;
  setWorkersSnapshot(payload.workers);
      setWorkerControlMessage(`Workers rafraichis: ${payload.workers.length}.`);
      log(`Workers rafraichis (${payload.workers.length})`, "ok");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setWorkerControlMessage("Erreur reseau lors du chargement workers.");
      log(`Chargement workers en echec: ${message}`, "error");
    }
  }, [log, setWorkerControlMessage, setWorkersSnapshot]);

  // Claims one pending build for selected worker.
  const claimBuildForWorker = useCallback(async () => {
    if (!roleCapabilities.can_run_operations) {
      setWorkerControlMessage("Role insuffisant pour claim build.");
      log(`Role ${adminRole} ne peut pas claim de build`, "warn");
      audit("worker_claim_denied", workerControlForm.worker_id || "unknown");
      return;
    }

    const workerId = workerControlForm.worker_id.trim();
    if (!workerId) {
      setWorkerControlMessage("Worker id requis pour claim.");
      log("Claim worker refuse: worker id manquant", "warn");
      return;
    }

    try {
      const response = await fetch(`/workers/${encodeURIComponent(workerId)}/claim`, {
        method: "POST"
      });

      if (!response.ok) {
        setWorkerControlMessage(`Claim en echec (HTTP ${response.status}).`);
        log(`Claim worker en echec (${workerId}): HTTP ${response.status}`, "error");
        return;
      }

      const payload = (await response.json()) as ClaimBuildResponse;
      if (!payload.build) {
        setLastClaimResult("Aucun build disponible.");
        setWorkerControlMessage("Claim termine: file vide.");
        log(`Claim worker ${workerId}: aucun build disponible`, "info");
        await refreshWorkers();
        return;
      }

      setWorkerControlForm((previous) => ({
        ...previous,
        build_id: payload.build?.id ?? previous.build_id
      }));
      setLastClaimResult(`Build ${payload.build.id.slice(0, 8)} claim.`);
      setWorkerControlMessage(`Claim reussi: build ${payload.build.id.slice(0, 8)}.`);
      log(`Claim worker ${workerId}: build ${payload.build.id.slice(0, 8)}`, "ok");
      audit("worker_claim", workerId);
      await refreshAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setWorkerControlMessage("Erreur reseau lors du claim worker.");
      log(`Claim worker en echec (${workerId}): ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    refreshAll,
    refreshWorkers,
    roleCapabilities.can_run_operations,
    setLastClaimResult,
    setWorkerControlForm,
    setWorkerControlMessage,
    workerControlForm.worker_id
  ]);

  // Completes one claimed build for selected worker.
  const completeBuildForWorker = useCallback(async () => {
    if (!roleCapabilities.can_run_operations) {
      setWorkerControlMessage("Role insuffisant pour complete build.");
      log(`Role ${adminRole} ne peut pas completer de build`, "warn");
      audit("worker_complete_denied", workerControlForm.build_id || "unknown");
      return;
    }

    const workerId = workerControlForm.worker_id.trim();
    const buildId = workerControlForm.build_id.trim();

    if (!workerId || !buildId) {
      setWorkerControlMessage("Worker id et build id requis pour completion.");
      log("Completion worker refusee: worker/build id manquant", "warn");
      return;
    }

    if (workerControlForm.completion_status === "failed") {
      if (!roleCapabilities.can_mutate_sensitive) {
        setWorkerControlMessage("Role insuffisant pour completion failed.");
        log(`Role ${adminRole} ne peut pas forcer un failed`, "warn");
        audit("worker_complete_failed_denied", buildId || "unknown");
        return;
      }

      const confirmed = globalThis.confirm(
        "Confirmer une completion en echec ? Cela peut declencher retry/dead-letter."
      );
      if (!confirmed) {
        setWorkerControlMessage("Completion en echec annulee.");
        return;
      }
    }

    const payload = {
      status: workerControlForm.completion_status,
      log_line: workerControlForm.completion_log_line.trim() || null
    };

    try {
      const response = await fetch(
        `/workers/${encodeURIComponent(workerId)}/builds/${encodeURIComponent(buildId)}/complete`,
        {
          method: "POST",
          headers: {
            "content-type": "application/json"
          },
          body: JSON.stringify(payload)
        }
      );

      if (response.status === 409) {
        setWorkerControlMessage("Conflit de possession: ce build appartient a un autre worker.");
        log(`Completion worker conflit (${workerId}, ${buildId.slice(0, 8)})`, "warn");
        return;
      }

      if (response.status === 400) {
        setWorkerControlMessage("Transition invalide pour completion worker.");
        log(`Completion worker invalide (${workerId}, ${buildId.slice(0, 8)})`, "warn");
        return;
      }

      if (!response.ok) {
        setWorkerControlMessage(`Completion en echec (HTTP ${response.status}).`);
        log(
          `Completion worker en echec (${workerId}, ${buildId.slice(0, 8)}): HTTP ${response.status}`,
          "error"
        );
        return;
      }

      const completion = (await response.json()) as CompleteBuildResponse;
      setWorkerControlMessage(
        `Completion reussie: build ${completion.build.id.slice(0, 8)} -> ${completion.build.status}.`
      );
      setWorkerControlForm((previous) => ({ ...previous, build_id: "", completion_log_line: "" }));
      log(
        `Completion worker ${workerId}: build ${completion.build.id.slice(0, 8)} -> ${completion.build.status}`,
        "ok"
      );
      audit("worker_complete", workerId);
      await refreshAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setWorkerControlMessage("Erreur reseau lors de la completion worker.");
      log(`Completion worker en echec (${workerId}, ${buildId.slice(0, 8)}): ${message}`, "error");
    }
  }, [
    adminRole,
    audit,
    log,
    refreshAll,
    roleCapabilities.can_mutate_sensitive,
    roleCapabilities.can_run_operations,
    setWorkerControlForm,
    setWorkerControlMessage,
    workerControlForm
  ]);

  return {
    refreshWorkers,
    claimBuildForWorker,
    completeBuildForWorker
  };
}