import { useOrchestratorStore } from "../stores/orchestrator";

export function StatusBar() {
  const tasks = useOrchestratorStore((s) => s.tasks);
  const isPlanning = useOrchestratorStore((s) => s.isPlanning);
  const isRunning = useOrchestratorStore((s) => s.isRunning);
  const workers = useOrchestratorStore((s) => s.workers);

  const completed = tasks.filter((t) => t.status === "completed").length;
  const failed = tasks.filter((t) => t.status === "failed").length;
  const running = tasks.filter((t) => t.status === "running").length;
  const total = tasks.length;
  const busyWorkers = workers.filter((w) => w.status === "busy").length;

  const status = isPlanning
    ? "规划中..."
    : isRunning
      ? `执行中 (${completed + failed}/${total})`
      : total > 0
        ? failed > 0
          ? `完成，${failed} 个失败`
          : "全部完成"
        : "空闲";

  const progressPct = total > 0 ? ((completed + failed) / total) * 100 : 0;

  return (
    <div
      className="flex items-center justify-between px-5 py-1.5 text-xs shrink-0"
      style={{
        background: "var(--bg-secondary)",
        borderTop: "1px solid var(--border)",
        color: "var(--text-muted)",
      }}
    >
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1.5">
          <span
            className="w-2 h-2 rounded-full inline-block"
            style={{
              background: isPlanning || isRunning ? "var(--warning)" : total > 0 && failed === 0 && completed === total ? "var(--success)" : failed > 0 ? "var(--error)" : "var(--text-muted)",
              animation: isPlanning || isRunning ? "pulse 1.5s infinite" : "none",
            }}
          />
          {status}
        </span>
        {isRunning && total > 0 && (
          <div className="flex items-center gap-2">
            <div
              className="h-1.5 rounded-full overflow-hidden"
              style={{ width: 100, background: "var(--bg-tertiary)" }}
            >
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${progressPct}%`,
                  background: failed > 0 ? "var(--error)" : "var(--accent)",
                }}
              />
            </div>
            <span>{Math.round(progressPct)}%</span>
          </div>
        )}
      </div>
      <div className="flex items-center gap-4">
        {total > 0 && (
          <span>
            {running > 0 && <span style={{ color: "var(--warning)" }}>{running} 执行中</span>}
            {running > 0 && completed > 0 && " / "}
            {completed > 0 && <span style={{ color: "var(--success)" }}>{completed} 已完成</span>}
            {(running > 0 || completed > 0) && failed > 0 && " / "}
            {failed > 0 && <span style={{ color: "var(--error)" }}>{failed} 失败</span>}
          </span>
        )}
        <span>工作者: {busyWorkers}/{workers.length} 忙碌</span>
      </div>
    </div>
  );
}
