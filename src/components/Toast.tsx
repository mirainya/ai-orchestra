import { useEffect } from "react";
import { CheckCircle2, XCircle, Info, X } from "lucide-react";
import { useOrchestratorStore } from "../stores/orchestrator";

export interface ToastItem {
  id: number;
  type: "success" | "error" | "info";
  message: string;
  exiting?: boolean;
}

const icons = {
  success: <CheckCircle2 size={16} />,
  error: <XCircle size={16} />,
  info: <Info size={16} />,
};

const colors = {
  success: "var(--success)",
  error: "var(--error)",
  info: "var(--accent)",
};

export function ToastContainer() {
  const toasts = useOrchestratorStore((s) => s.toasts);
  const removeToast = useOrchestratorStore((s) => s.removeToast);
  const markToastExiting = useOrchestratorStore((s) => s.markToastExiting);

  return (
    <div style={{ position: "fixed", top: 16, right: 16, zIndex: 100, display: "flex", flexDirection: "column", gap: 8 }}>
      {toasts.map((t) => (
        <ToastEntry key={t.id} toast={t} onClose={() => removeToast(t.id)} onStartExit={() => markToastExiting(t.id)} />
      ))}
    </div>
  );
}

function ToastEntry({ toast, onClose, onStartExit }: { toast: ToastItem; onClose: () => void; onStartExit: () => void }) {
  useEffect(() => {
    const timer = setTimeout(onStartExit, 3000);
    return () => clearTimeout(timer);
  }, [onStartExit]);

  useEffect(() => {
    if (toast.exiting) {
      const timer = setTimeout(onClose, 300);
      return () => clearTimeout(timer);
    }
  }, [toast.exiting, onClose]);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "10px 14px",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-secondary)",
        border: "1px solid var(--border)",
        boxShadow: "var(--shadow-md)",
        color: colors[toast.type],
        fontSize: 13,
        minWidth: 240,
        maxWidth: 360,
        animation: toast.exiting ? "toast-out 300ms ease-in forwards" : "toast-in 300ms ease-out",
      }}
    >
      {icons[toast.type]}
      <span style={{ flex: 1, color: "var(--text-primary)" }}>{toast.message}</span>
      <button
        onClick={() => { onStartExit(); }}
        style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: 2 }}
      >
        <X size={14} />
      </button>
    </div>
  );
}
