import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useOrchestratorStore } from "../stores/orchestrator";

const modeLabel: Record<string, string> = {
  cli: "CLI",
  api: "API",
};

interface WorkerConfig {
  name: string;
  cli_type: string;
  mode: string;
  role: string;
  skills: string[];
  cli_path: string | null;
  extra_args: string[];
  api_base_url: string | null;
  api_key: string | null;
  model: string | null;
}

interface ExecutionConfig {
  task_timeout_secs: number;
  max_retries: number;
  retry_delay_secs: number;
  planner_timeout_secs: number;
}

interface AppConfig {
  workers: WorkerConfig[];
  execution: ExecutionConfig;
}

export function ConfigPanel() {
  const setWorkers = useOrchestratorStore((s) => s.setWorkers);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      invoke<AppConfig>("get_config"),
      invoke<Array<{ name: string; cli_type: string; status: string; role: string; skills: string[] }>>("get_workers"),
    ])
      .then(([cfg, ws]) => {
        setConfig(cfg);
        setWorkers(
          ws.map((w) => ({
            name: w.name,
            cli_type: w.cli_type,
            status: w.status as "idle" | "busy",
            role: w.role,
            skills: w.skills,
          }))
        );
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [setWorkers]);

  if (loading) {
    return (
      <div className="text-xs" style={{ color: "var(--text-muted)" }}>
        加载配置...
      </div>
    );
  }

  if (!config) {
    return (
      <div className="text-xs" style={{ color: "var(--error)" }}>
        加载配置失败
      </div>
    );
  }

  const planner = config.workers.find((w) => w.role === "planner" || w.role === "both");
  const executors = config.workers.filter((w) => w.role === "executor" || w.role === "both");

  return (
    <div className="space-y-4">
      {/* Planner */}
      {planner && (
        <section>
          <h3
            className="text-xs font-semibold uppercase tracking-wide mb-2"
            style={{ color: "var(--text-muted)" }}
          >
            规划器
          </h3>
          <div
            className="rounded-lg p-3 space-y-1"
            style={{ background: "var(--bg-tertiary)" }}
          >
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
                {planner.name}
              </span>
              <span
                className="text-xs px-2 py-0.5 rounded"
                style={{ background: "var(--bg-secondary)", color: "var(--accent)" }}
              >
                {modeLabel[planner.mode] ?? planner.mode}
              </span>
            </div>
            <ConfigRow label="类型" value={planner.cli_type} />
            {planner.mode === "cli" && planner.cli_path && (
              <ConfigRow label="路径" value={planner.cli_path} />
            )}
            {planner.mode === "api" && planner.model && (
              <ConfigRow label="模型" value={planner.model} />
            )}
          </div>
        </section>
      )}

      {/* Executors */}
      <section>
        <h3
          className="text-xs font-semibold uppercase tracking-wide mb-2"
          style={{ color: "var(--text-muted)" }}
        >
          执行者 ({executors.length})
        </h3>
        <div className="space-y-2">
          {executors.map((w) => (
            <div
              key={w.name}
              className="rounded-lg p-3 space-y-1"
              style={{ background: "var(--bg-tertiary)" }}
            >
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
                  {w.name}
                </span>
                <div className="flex items-center gap-1">
                  <span
                    className="text-xs px-1.5 py-0.5 rounded"
                    style={{ background: "var(--bg-secondary)", color: "var(--accent)" }}
                  >
                    {modeLabel[w.mode] ?? w.mode}
                  </span>
                  <span
                    className="text-xs px-1.5 py-0.5 rounded"
                    style={{ background: "var(--bg-secondary)", color: "var(--text-muted)" }}
                  >
                    {w.cli_type}
                  </span>
                </div>
              </div>
              {w.mode === "cli" && w.cli_path && (
                <ConfigRow label="路径" value={w.cli_path} />
              )}
              {w.mode === "api" && w.model && (
                <ConfigRow label="模型" value={w.model} />
              )}
              {w.skills.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-1">
                  {w.skills.map((sk) => (
                    <span
                      key={sk}
                      className="text-xs px-1.5 py-0.5 rounded"
                      style={{ background: "var(--bg-primary)", color: "var(--text-secondary)", fontSize: 10 }}
                    >
                      {sk}
                    </span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      </section>

      {/* Execution Config */}
      <section>
        <h3
          className="text-xs font-semibold uppercase tracking-wide mb-2"
          style={{ color: "var(--text-muted)" }}
        >
          执行参数
        </h3>
        <div
          className="rounded-lg p-3 space-y-1"
          style={{ background: "var(--bg-tertiary)" }}
        >
          <ConfigRow label="超时" value={`${config.execution.task_timeout_secs}s`} />
          <ConfigRow label="重试" value={`${config.execution.max_retries} 次`} />
          <ConfigRow label="规划超时" value={`${config.execution.planner_timeout_secs}s`} />
        </div>
      </section>

      <div className="text-xs" style={{ color: "var(--text-muted)" }}>
        点击右上角齿轮图标进行详细配置
      </div>
    </div>
  );
}

function ConfigRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start gap-2">
      <span className="text-xs shrink-0" style={{ color: "var(--text-muted)", minWidth: 40 }}>
        {label}
      </span>
      <span
        className="text-xs font-mono break-all"
        style={{ color: "var(--text-secondary)" }}
      >
        {value || "-"}
      </span>
    </div>
  );
}
