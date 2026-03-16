import { useEffect, useRef, useState } from "react";
import { useOrchestratorStore, type OutputLine } from "../stores/orchestrator";

function formatTime(ts: number) {
  const d = new Date(ts);
  return d.toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

const levelStyle: Record<OutputLine["level"], string> = {
  info: "var(--text-secondary)",
  error: "var(--error)",
  success: "var(--success)",
  warn: "var(--warning)",
};

export function OutputPanel() {
  const lines = useOrchestratorStore((s) => s.outputLines);
  const clearOutput = useOrchestratorStore((s) => s.clearOutput);
  const selectedTaskId = useOrchestratorStore((s) => s.selectedTaskId);
  const bottomRef = useRef<HTMLDivElement>(null);
  const [filter, setFilter] = useState<string>("all");
  const [autoScroll, setAutoScroll] = useState(true);

  const sources = Array.from(new Set(lines.map((l) => l.source)));

  const activeFilter = selectedTaskId ?? filter;
  const filtered = activeFilter === "all"
    ? lines
    : lines.filter((l) => l.source === activeFilter);

  useEffect(() => {
    if (autoScroll) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [filtered, autoScroll]);

  return (
    <div
      className="rounded-xl flex flex-col"
      style={{ background: "var(--bg-secondary)", minHeight: 160, maxHeight: 280 }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-2 shrink-0"
        style={{ borderBottom: "1px solid var(--border)" }}
      >
        <div className="flex items-center gap-3">
          <h2
            className="text-sm font-semibold uppercase tracking-wide"
            style={{ color: "var(--text-muted)" }}
          >
            输出
          </h2>
          <span style={{ color: "var(--text-muted)", fontSize: 11 }}>
            {filtered.length} 行
          </span>
        </div>
        <div className="flex items-center gap-2">
          <select
            value={activeFilter}
            onChange={(e) => setFilter(e.target.value)}
            className="text-xs px-2 py-1 rounded"
            style={{
              background: "var(--bg-tertiary)",
              color: "var(--text-primary)",
              border: "1px solid var(--border)",
            }}
          >
            <option value="all">全部来源</option>
            {sources.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </select>
          <button
            onClick={() => setAutoScroll(!autoScroll)}
            className="text-xs px-2 py-1 rounded"
            style={{
              background: autoScroll ? "var(--accent)" : "var(--bg-tertiary)",
              color: autoScroll ? "#fff" : "var(--text-secondary)",
              border: "1px solid var(--border)",
            }}
            title={autoScroll ? "自动滚动 开" : "自动滚动 关"}
          >
            {autoScroll ? "\u2193 自动" : "\u2193 手动"}
          </button>
          <button
            onClick={clearOutput}
            className="text-xs px-2 py-1 rounded"
            style={{
              background: "var(--bg-tertiary)",
              color: "var(--text-secondary)",
              border: "1px solid var(--border)",
            }}
          >
            清空
          </button>
        </div>
      </div>

      {/* Lines */}
      <div
        className="flex-1 overflow-y-auto font-mono text-xs leading-5 px-4 py-2"
      >
        {filtered.length === 0 ? (
          <span style={{ color: "var(--text-muted)" }}>等待输出...</span>
        ) : (
          filtered.map((l, i) => (
            <div key={i} className="flex gap-2 hover:opacity-80" style={{ color: levelStyle[l.level] }}>
              <span style={{ color: "var(--text-muted)", flexShrink: 0 }}>
                {formatTime(l.timestamp)}
              </span>
              <span
                style={{
                  color: "var(--accent)",
                  flexShrink: 0,
                  minWidth: 70,
                  fontWeight: 600,
                }}
              >
                [{l.source}]
              </span>
              <span style={{ wordBreak: "break-all" }}>{l.text}</span>
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
