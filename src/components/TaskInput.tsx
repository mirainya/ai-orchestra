import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, X, Sparkles, Loader2 } from "lucide-react";
import { useOrchestratorStore } from "../stores/orchestrator";

export function TaskInput() {
  const [input, setInput] = useState("");
  const {
    isPlanning,
    isRunning,
    workingDir,
    plannerSessionId,
    setWorkingDir,
    setPlanning,
    setGoal,
    reset,
    appendOutput,
    appendPlannerMessage,
    setPlannerSessionId,
    setPlannerStatus,
  } = useOrchestratorStore();

  const busy = isPlanning || isRunning;

  const handlePickDir = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setWorkingDir(selected);
    }
  };

  const handleSubmit = async () => {
    const trimmed = input.trim();
    if (!trimmed || busy) return;
    reset();
    setGoal(trimmed);
    setPlanning(true);
    appendPlannerMessage({ role: "user", content: trimmed });
    setPlannerStatus("thinking");
    try {
      const sessionId = await invoke<string>("start_planning", {
        goal: trimmed,
        workingDir: workingDir ?? undefined,
      });
      setPlannerSessionId(sessionId);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      appendOutput("system", `Planning failed: ${msg}`, "error");
      setPlannerStatus("idle");
    } finally {
      setPlanning(false);
    }
    setInput("");
  };

  // Show abbreviated path
  const dirLabel = workingDir
    ? workingDir.length > 30
      ? "..." + workingDir.slice(-27)
      : workingDir
    : null;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex gap-2">
        <button
          onClick={handlePickDir}
          disabled={busy}
          title={workingDir ?? "选择工作目录"}
          className="flex items-center gap-1.5 px-3 py-2.5 rounded-lg text-sm transition-colors shrink-0 disabled:opacity-50"
          style={{
            background: workingDir ? "var(--accent)" : "var(--bg-tertiary)",
            color: workingDir ? "#fff" : "var(--text-secondary)",
            border: "1px solid var(--border)",
          }}
        >
          <FolderOpen size={16} />
          {dirLabel ?? "工作目录"}
        </button>
        {workingDir && (
          <button
            onClick={() => setWorkingDir(null)}
            disabled={busy}
            className="px-2 py-2.5 rounded-lg text-xs transition-colors disabled:opacity-50"
            style={{
              background: "var(--bg-tertiary)",
              color: "var(--text-muted)",
              border: "1px solid var(--border)",
            }}
            title="清除工作目录"
          >
            <X size={14} />
          </button>
        )}
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
          placeholder="请输入任务描述...（例如：'创建一个带用户认证的 REST API'）"
          disabled={busy || !!plannerSessionId}
          className="flex-1 px-4 py-2.5 rounded-lg outline-none text-sm"
          style={{
            background: "var(--bg-tertiary)",
            color: "var(--text-primary)",
            border: "1px solid var(--border)",
          }}
        />
        <button
          onClick={handleSubmit}
          disabled={busy || !input.trim() || !!plannerSessionId}
          className="px-6 py-2.5 rounded-lg font-medium text-sm transition-colors disabled:opacity-50"
          style={{
            background: "var(--accent)",
            color: "#fff",
          }}
        >
          {isPlanning ? <Loader2 size={16} className="animate-spin" /> : isRunning ? "执行中..." : <><Sparkles size={16} /> 规划</>}
        </button>
      </div>
    </div>
  );
}
