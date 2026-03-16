import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, Trash2, Clock } from "lucide-react";

interface HistorySummary {
  id: string;
  goal: string;
  started_at: string;
  finished_at: string | null;
  status: "running" | "completed" | "failed";
  task_count: number;
}

const statusStyle: Record<string, { color: string; label: string }> = {
  running: { color: "var(--warning)", label: "执行中" },
  completed: { color: "var(--success)", label: "已完成" },
  failed: { color: "var(--error)", label: "失败" },
};

export function HistoryPanel() {
  const [entries, setEntries] = useState<HistorySummary[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = () => {
    setLoading(true);
    invoke<HistorySummary[]>("get_history_list")
      .then(setEntries)
      .catch(console.error)
      .finally(() => setLoading(false));
  };

  useEffect(() => { refresh(); }, []);

  const handleDelete = async (id: string) => {
    await invoke("delete_history_entry", { id }).catch(console.error);
    refresh();
  };

  if (loading) {
    return <div className="text-xs p-2" style={{ color: "var(--text-muted)" }}>加载中...</div>;
  }

  if (entries.length === 0) {
    return (
      <div className="text-xs p-2" style={{ color: "var(--text-muted)" }}>
        暂无执行历史
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between mb-2">
        <h3
          className="text-xs font-semibold uppercase tracking-wide"
          style={{ color: "var(--text-muted)" }}
        >
          历史 ({entries.length})
        </h3>
        <button
          onClick={refresh}
          className="text-xs px-2 py-0.5 rounded"
          style={{ background: "var(--bg-tertiary)", color: "var(--text-secondary)" }}
        >
          <RefreshCw size={12} />
        </button>
      </div>
      {entries.map((e) => {
        const s = statusStyle[e.status] ?? statusStyle.running;
        return (
          <div
            key={e.id}
            className="rounded-lg p-3 space-y-1"
            style={{
              background: "var(--bg-tertiary)",
              boxShadow: "var(--shadow-sm)",
              transition: "box-shadow var(--transition-normal)",
            }}
            onMouseEnter={(ev) => (ev.currentTarget.style.boxShadow = "var(--shadow-md)")}
            onMouseLeave={(ev) => (ev.currentTarget.style.boxShadow = "var(--shadow-sm)")}
          >
            <div className="flex items-center justify-between">
              <span
                className="text-xs font-semibold uppercase"
                style={{ color: s.color }}
              >
                {s.label}
              </span>
              <button
                onClick={() => handleDelete(e.id)}
                className="text-xs px-1.5 py-0.5 rounded opacity-60 hover:opacity-100"
                style={{ color: "var(--error)" }}
              >
                <Trash2 size={12} />
              </button>
            </div>
            <div
              className="text-sm"
              style={{
                color: "var(--text-primary)",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
              title={e.goal}
            >
              {e.goal}
            </div>
            <div className="flex items-center gap-2 text-xs" style={{ color: "var(--text-muted)" }}>
              <span className="flex items-center gap-1"><Clock size={10} />{e.task_count} 个任务</span>
              <span>{e.started_at}</span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
