import { Bot } from "lucide-react";
import { useOrchestratorStore } from "../stores/orchestrator";

const roleLabel: Record<string, string> = {
  planner: "规划者",
  executor: "执行者",
  both: "双角色",
};

export function WorkerPanel() {
  const workers = useOrchestratorStore((s) => s.workers);

  return (
    <div className="space-y-2">
      <h2
        className="text-xs font-semibold uppercase tracking-wide mb-2"
        style={{ color: "var(--text-muted)" }}
      >
        工作者
      </h2>
      {workers.length === 0 ? (
        <div style={{ color: "var(--text-muted)", fontSize: 13 }}>
          暂无工作者配置
        </div>
      ) : (
        workers.map((w) => (
          <div
            key={w.name}
            className="px-3 py-2 rounded-lg space-y-1.5"
            style={{
              background: "var(--bg-tertiary)",
              boxShadow: "var(--shadow-sm)",
              transition: "box-shadow var(--transition-normal)",
            }}
            onMouseEnter={(e) => (e.currentTarget.style.boxShadow = "var(--shadow-md)")}
            onMouseLeave={(e) => (e.currentTarget.style.boxShadow = "var(--shadow-sm)")}
          >
            <div className="flex items-center justify-between">
              <div className="text-sm font-medium flex items-center gap-1.5" style={{ color: "var(--text-primary)" }}>
                <Bot size={14} style={{ color: "var(--accent)" }} />
                {w.name}
              </div>
              <span
                className="text-xs px-2 py-0.5 rounded-full font-medium"
                style={{
                  background: w.status === "busy" ? "var(--warning)" : "var(--success)",
                  color: "#000",
                }}
              >
                {w.status === "busy" ? "忙碌" : "空闲"}
              </span>
            </div>
            <div className="flex items-center gap-2 flex-wrap">
              <span
                className="text-xs px-1.5 py-0.5 rounded"
                style={{ background: "var(--bg-secondary)", color: "var(--text-muted)" }}
              >
                {w.cli_type}
              </span>
              {w.role && (
                <span
                  className="text-xs px-1.5 py-0.5 rounded"
                  style={{ background: "var(--bg-secondary)", color: "var(--accent)" }}
                >
                  {roleLabel[w.role] ?? w.role}
                </span>
              )}
            </div>
            {w.skills && w.skills.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {w.skills.map((sk) => (
                  <span
                    key={sk}
                    className="text-xs px-1.5 py-0.5 rounded"
                    style={{
                      background: "var(--bg-primary)",
                      color: "var(--text-secondary)",
                      fontSize: 10,
                    }}
                  >
                    {sk}
                  </span>
                ))}
              </div>
            )}
          </div>
        ))
      )}
    </div>
  );
}
