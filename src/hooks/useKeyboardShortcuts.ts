import { useEffect } from "react";
import { useOrchestratorStore } from "../stores/orchestrator";

export function useKeyboardShortcuts(callbacks: {
  onSubmit?: () => void;
  onCloseSettings?: () => void;
  onClearOutput?: () => void;
}) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ctrl+Enter — submit task
      if (e.ctrlKey && e.key === "Enter") {
        e.preventDefault();
        callbacks.onSubmit?.();
      }
      // Escape — close settings / task detail
      if (e.key === "Escape") {
        callbacks.onCloseSettings?.();
        useOrchestratorStore.getState().setSelectedTaskId(null);
      }
      // Ctrl+L — clear output
      if (e.ctrlKey && e.key === "l") {
        e.preventDefault();
        callbacks.onClearOutput?.();
        useOrchestratorStore.getState().clearOutput();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [callbacks]);
}
