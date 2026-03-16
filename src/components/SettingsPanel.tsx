import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface WorkerConfig {
  name: string;
  cli_type: string;
  mode: "cli" | "api";
  role: "planner" | "executor" | "both";
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

const CLI_TYPES = [
  { value: "claude_cli", label: "Claude CLI" },
  { value: "codex_cli", label: "Codex CLI" },
  { value: "glm_cli", label: "GLM CLI" },
  { value: "openai", label: "OpenAI API" },
  { value: "anthropic", label: "Anthropic API" },
];

const ROLES = [
  { value: "planner", label: "规划者" },
  { value: "executor", label: "执行者" },
  { value: "both", label: "两者皆可" },
];

const SKILL_PRESETS = ["coding", "writing", "analysis", "testing", "review", "planning"];

const inputCls = "w-full text-xs px-2 py-1.5 rounded outline-none";

function inputStyle() {
  return {
    background: "var(--bg-primary)",
    color: "var(--text-primary)",
    border: "1px solid var(--border)",
  };
}

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    invoke<AppConfig>("get_config")
      .then(setConfig)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const save = async () => {
    if (!config) return;
    setSaving(true);
    try {
      await invoke("save_config", { config });
      setDirty(false);
    } catch (e) {
      console.error(e);
    } finally {
      setSaving(false);
    }
  };

  const updateWorker = (idx: number, patch: Partial<WorkerConfig>) => {
    if (!config) return;
    const workers = [...config.workers];
    workers[idx] = { ...workers[idx], ...patch };
    setConfig({ ...config, workers });
    setDirty(true);
  };

  const addWorker = () => {
    if (!config) return;
    const w: WorkerConfig = {
      name: `worker-${config.workers.length + 1}`,
      cli_type: "codex_cli",
      mode: "cli",
      role: "executor",
      skills: ["coding"],
      cli_path: "codex",
      extra_args: [],
      api_base_url: null,
      api_key: null,
      model: null,
    };
    setConfig({ ...config, workers: [...config.workers, w] });
    setDirty(true);
  };

  const removeWorker = (idx: number) => {
    if (!config) return;
    const workers = config.workers.filter((_, i) => i !== idx);
    setConfig({ ...config, workers });
    setDirty(true);
  };

  const updateExecution = (patch: Partial<ExecutionConfig>) => {
    if (!config) return;
    setConfig({ ...config, execution: { ...config.execution, ...patch } });
    setDirty(true);
  };

  if (loading) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: "rgba(0,0,0,0.5)" }}>
        <div className="rounded-xl p-6" style={{ background: "var(--bg-secondary)", color: "var(--text-muted)" }}>
          加载中...
        </div>
      </div>
    );
  }

  if (!config) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: "rgba(0,0,0,0.5)" }}>
      <div
        className="rounded-xl flex flex-col"
        style={{
          background: "var(--bg-secondary)",
          width: 640,
          maxHeight: "85vh",
          border: "1px solid var(--border)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-5 py-3 shrink-0"
          style={{ borderBottom: "1px solid var(--border)" }}
        >
          <h2 className="text-sm font-bold" style={{ color: "var(--text-primary)" }}>
            设置
          </h2>
          <div className="flex items-center gap-2">
            {dirty && (
              <button
                onClick={save}
                disabled={saving}
                className="text-xs px-3 py-1.5 rounded font-medium disabled:opacity-50"
                style={{ background: "var(--accent)", color: "#fff" }}
              >
                {saving ? "保存中..." : "保存"}
              </button>
            )}
            <button
              onClick={onClose}
              className="text-xs px-3 py-1.5 rounded"
              style={{ background: "var(--bg-tertiary)", color: "var(--text-secondary)" }}
            >
              关闭
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5 space-y-5">
          {/* Execution Config */}
          <section>
            <h3
              className="text-xs font-semibold uppercase tracking-wide mb-3"
              style={{ color: "var(--text-muted)" }}
            >
              执行配置
            </h3>
            <div className="grid grid-cols-2 gap-3">
              <Field label="任务超时 (秒)">
                <input
                  type="number"
                  value={config.execution.task_timeout_secs}
                  onChange={(e) => updateExecution({ task_timeout_secs: +e.target.value })}
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
              <Field label="最大重试次数">
                <input
                  type="number"
                  value={config.execution.max_retries}
                  onChange={(e) => updateExecution({ max_retries: +e.target.value })}
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
              <Field label="重试延迟 (秒)">
                <input
                  type="number"
                  value={config.execution.retry_delay_secs}
                  onChange={(e) => updateExecution({ retry_delay_secs: +e.target.value })}
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
              <Field label="规划超时 (秒)">
                <input
                  type="number"
                  value={config.execution.planner_timeout_secs}
                  onChange={(e) => updateExecution({ planner_timeout_secs: +e.target.value })}
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
            </div>
          </section>

          {/* Workers */}
          <section>
            <div className="flex items-center justify-between mb-3">
              <h3
                className="text-xs font-semibold uppercase tracking-wide"
                style={{ color: "var(--text-muted)" }}
              >
                工作者 ({config.workers.length})
              </h3>
              <button
                onClick={addWorker}
                className="text-xs px-2 py-1 rounded"
                style={{ background: "var(--accent)", color: "#fff" }}
              >
                + 添加
              </button>
            </div>

            <div className="space-y-3">
              {config.workers.map((w, i) => (
                <WorkerEditor
                  key={i}
                  worker={w}
                  onChange={(patch) => updateWorker(i, patch)}
                  onRemove={() => removeWorker(i)}
                />
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}

function WorkerEditor({
  worker,
  onChange,
  onRemove,
}: {
  worker: WorkerConfig;
  onChange: (patch: Partial<WorkerConfig>) => void;
  onRemove: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string; latency_ms: number } | null>(null);
  const isApi = worker.mode === "api";

  const runTest = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setTesting(true);
    setTestResult(null);
    try {
      const result = await invoke<{ success: boolean; message: string; latency_ms: number }>("test_worker", { worker });
      setTestResult(result);
    } catch (err) {
      setTestResult({ success: false, message: String(err), latency_ms: 0 });
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="rounded-lg" style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border)" }}>
      {/* Summary row */}
      <div
        className="flex items-center justify-between px-3 py-2 cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
            {worker.name}
          </span>
          <span
            className="text-xs px-1.5 py-0.5 rounded"
            style={{ background: "var(--bg-secondary)", color: "var(--accent)" }}
          >
            {worker.mode === "api" ? "API" : "CLI"}
          </span>
          <span
            className="text-xs px-1.5 py-0.5 rounded"
            style={{
              background: "var(--bg-secondary)",
              color: worker.role === "planner" ? "var(--warning)" : "var(--success)",
            }}
          >
            {ROLES.find((r) => r.value === worker.role)?.label ?? worker.role}
          </span>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={runTest}
            disabled={testing}
            className="text-xs px-1.5 py-0.5 rounded opacity-80 hover:opacity-100 disabled:opacity-40"
            style={{ background: "var(--bg-secondary)", color: "var(--accent)", border: "1px solid var(--border)" }}
          >
            {testing ? "测试中..." : "测试"}
          </button>
          <button
            onClick={(e) => { e.stopPropagation(); onRemove(); }}
            className="text-xs px-1.5 py-0.5 rounded opacity-60 hover:opacity-100"
            style={{ color: "var(--error)" }}
          >
            删除
          </button>
          <span style={{ color: "var(--text-muted)", fontSize: 10 }}>
            {expanded ? "▲" : "▼"}
          </span>
        </div>
      </div>
      {testResult && (
        <div
          className="px-3 py-1.5 text-xs flex items-center gap-2"
          style={{
            borderTop: "1px solid var(--border)",
            background: testResult.success ? "rgba(34,197,94,0.08)" : "rgba(239,68,68,0.08)",
            color: testResult.success ? "var(--success)" : "var(--error)",
            borderRadius: "0 0 8px 8px",
          }}
        >
          <span>{testResult.success ? "✓" : "✗"}</span>
          <span>{testResult.message}</span>
          {testResult.latency_ms > 0 && (
            <span style={{ color: "var(--text-muted)", marginLeft: "auto" }}>
              {testResult.latency_ms}ms
            </span>
          )}
        </div>
      )}

      {/* Expanded editor */}
      {expanded && (
        <div className="px-3 pb-3 space-y-2" style={{ borderTop: "1px solid var(--border)" }}>
          <div className="grid grid-cols-2 gap-2 pt-2">
            <Field label="名称">
              <input
                value={worker.name}
                onChange={(e) => onChange({ name: e.target.value })}
                className={inputCls}
                style={inputStyle()}
              />
            </Field>
            <Field label="类型">
              <select
                value={worker.cli_type}
                onChange={(e) => {
                  const v = e.target.value;
                  const autoMode = ["openai", "anthropic"].includes(v) ? "api" : "cli";
                  onChange({ cli_type: v, mode: autoMode as "cli" | "api" });
                }}
                className={inputCls}
                style={inputStyle()}
              >
                {CLI_TYPES.map((t) => (
                  <option key={t.value} value={t.value}>{t.label}</option>
                ))}
              </select>
            </Field>
            <Field label="模式">
              <select
                value={worker.mode}
                onChange={(e) => onChange({ mode: e.target.value as "cli" | "api" })}
                className={inputCls}
                style={inputStyle()}
              >
                <option value="cli">CLI 子进程</option>
                <option value="api">HTTP API</option>
              </select>
            </Field>
            <Field label="角色">
              <select
                value={worker.role}
                onChange={(e) => onChange({ role: e.target.value as WorkerConfig["role"] })}
                className={inputCls}
                style={inputStyle()}
              >
                {ROLES.map((r) => (
                  <option key={r.value} value={r.value}>{r.label}</option>
                ))}
              </select>
            </Field>
          </div>

          {/* Mode-specific fields */}
          {isApi ? (
            <div className="grid grid-cols-2 gap-2">
              <Field label="API Base URL" span={2}>
                <input
                  value={worker.api_base_url ?? ""}
                  onChange={(e) => onChange({ api_base_url: e.target.value || null })}
                  placeholder="https://api.openai.com"
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
              <Field label="API Key">
                <input
                  type="password"
                  value={worker.api_key ?? ""}
                  onChange={(e) => onChange({ api_key: e.target.value || null })}
                  placeholder="sk-..."
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
              <Field label="模型">
                <input
                  value={worker.model ?? ""}
                  onChange={(e) => onChange({ model: e.target.value || null })}
                  placeholder="gpt-4o"
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-2">
              <Field label="CLI 路径">
                <input
                  value={worker.cli_path ?? ""}
                  onChange={(e) => onChange({ cli_path: e.target.value || null })}
                  placeholder="codex"
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
              <Field label="额外参数">
                <input
                  value={worker.extra_args.join(" ")}
                  onChange={(e) => onChange({ extra_args: e.target.value ? e.target.value.split(" ") : [] })}
                  placeholder="--flag value"
                  className={inputCls}
                  style={inputStyle()}
                />
              </Field>
            </div>
          )}

          {/* Skills */}
          <Field label="技能标签">
            <div className="flex flex-wrap gap-1">
              {SKILL_PRESETS.map((sk) => {
                const active = worker.skills.includes(sk);
                return (
                  <button
                    key={sk}
                    onClick={() => {
                      const skills = active
                        ? worker.skills.filter((s) => s !== sk)
                        : [...worker.skills, sk];
                      onChange({ skills });
                    }}
                    className="text-xs px-2 py-0.5 rounded transition-colors"
                    style={{
                      background: active ? "var(--accent)" : "var(--bg-secondary)",
                      color: active ? "#fff" : "var(--text-muted)",
                      border: "1px solid var(--border)",
                    }}
                  >
                    {sk}
                  </button>
                );
              })}
            </div>
          </Field>
        </div>
      )}
    </div>
  );
}

function Field({ label, children, span }: { label: string; children: React.ReactNode; span?: number }) {
  return (
    <div style={span === 2 ? { gridColumn: "span 2" } : undefined}>
      <label className="text-xs block mb-1" style={{ color: "var(--text-muted)" }}>
        {label}
      </label>
      {children}
    </div>
  );
}
