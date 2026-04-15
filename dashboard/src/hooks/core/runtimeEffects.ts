import { useCallback, useEffect } from "react";

import type { HealthStatus, LiveEvent } from "../dashboardTypes";
import { stardateValue } from "../dashboardUtils";

interface RuntimeEffectsParams {
  log: (message: string, kind?: string) => void;
  refreshAll: () => Promise<void>;
  refreshTimerRef: { current: number | null };
  streamConnected: boolean;
  setStreamConnected: React.Dispatch<React.SetStateAction<boolean>>;
  setHealthStatus: React.Dispatch<React.SetStateAction<HealthStatus>>;
  setStardate: React.Dispatch<React.SetStateAction<string>>;
  pushLiveEvent: (event: LiveEvent) => void;
  scheduleRefresh: (delayMs?: number) => void;
}

// Runs the health polling, realtime stream, and timer lifecycle effects for the dashboard.
export function useRuntimeEffects({
  log,
  refreshAll,
  refreshTimerRef,
  streamConnected,
  setStreamConnected,
  setHealthStatus,
  setStardate,
  pushLiveEvent,
  scheduleRefresh
}: Readonly<RuntimeEffectsParams>) {
  // Reads the health endpoint to display backend availability in the header.
  const refreshHealth = useCallback(async () => {
    try {
      const response = await fetch("/health", { method: "GET" });
      setHealthStatus(response.ok ? "ok" : "degraded");
    } catch {
      setHealthStatus("degraded");
    }
  }, [setHealthStatus]);

  // Initializes dashboard data and baseline log once on first mount.
  useEffect(() => {
    log("Console initialisee", "ok");
    void refreshAll();
    void refreshHealth();
  }, [log, refreshAll, refreshHealth]);

  // Polls /health so the top HUD reflects backend availability.
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      void refreshHealth();
    }, 5000);
    return () => globalThis.clearInterval(id);
  }, [refreshHealth]);

  // Keeps the stardate indicator updated each minute.
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      setStardate(stardateValue(new Date()));
    }, 60000);
    return () => globalThis.clearInterval(id);
  }, [setStardate]);

  // Polling fallback ensures updates continue while SSE is disconnected.
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      if (!streamConnected) {
        void refreshAll();
      }
    }, 5000);
    return () => globalThis.clearInterval(id);
  }, [streamConnected, refreshAll]);

  // Opens the SSE stream and wires realtime events to logs and snapshot refresh.
  useEffect(() => {
    if (globalThis.EventSource === undefined) {
      log("EventSource non supporte, mode polling uniquement", "warn");
      return;
    }

    const source = new EventSource("/events");

    source.onopen = () => {
      setStreamConnected(true);
      log("Flux temps reel connecte", "ok");
    };

    source.onerror = () => {
      setStreamConnected((previous) => {
        if (previous) {
          log("Perte du flux temps reel, reconnexion en cours", "warn");
        }
        return false;
      });
    };

    source.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data) as LiveEvent;
        pushLiveEvent(payload);
        scheduleRefresh(80);
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        log(`Evenement live invalide: ${message}`, "error");
      }
    };

    return () => {
      source.close();
    };
  }, [log, pushLiveEvent, scheduleRefresh, setStreamConnected]);

  // Clears any pending debounced refresh timer on unmount.
  useEffect(() => {
    return () => {
      if (refreshTimerRef.current) {
        globalThis.clearTimeout(refreshTimerRef.current);
      }
    };
  }, [refreshTimerRef]);
}