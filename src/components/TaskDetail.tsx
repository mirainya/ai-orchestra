import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Send } from "lucide-react";
import { useOrchestratorStore } from "../stores/orchestrator";

const statusLabel: Record<string, string> = {
  pending: "待执行",
  running: "执行中",
  completed: "已完成",
  failed: "失败",
};

export function TaskDetail() {
  const [chatInput, setChatInput] = useState("");
  const [isSending, setIsSending] = useState(false);
  const chatEndRef = useRef<HTMLDivElement>(null);

  const selectedTaskId = useOrchestratorStore((s) => s.selectedTaskId);
  const tasks = useOrchestratorStore((s) => s.tasks);
  const taskSessions = useOrchestratorStore((s) => s.taskSessions);
  const setSelectedTaskId = useOrchestratorStore((s) => s.setSelectedTaskId);
  const appendOutput = useOrchestratorStore((s) => s.appendOutput);
  const appendTaskMessage = useOrchestratorStore((s) => s.appendTaskMessage);

  const task = tasks.find((t) => t.id === selectedTaskId);
  const chatMessages = task ? (taskSessions[task.id] ?? []) : [];

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatMessages]);

  if (!task) return null;

  const statusColor: Record<string, string> = {
    pending: "var(--text-muted)",
    running: "var(--warning)",
    completed: "var(--success)",
    failed: "var(--error)",
  };

  const handleSendMessage = async () => {
    const trimmed = chatInput.trim();
    if (!trimmed || !task.session_id || isSending) return;
    setChatInput("");
    setIsSending(true);
    appendTaskMessage(task.id, { role: "user", content: trimmed });
    try {
      await invoke("send_task_message", {
        sessionId: task.session_id,
        taskId: task.id,
        message: trimmed,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      appendOutput(task.id, `Chat error: ${msg}`, "error");
    } finally {
      setIsSending(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 overflow-y-auto space-y-3 p-0" style={{ minHeight: 0 }}>
        <div className="flex items-center justify-between">
          <span className="text-xs font-mono" style={{ color: "var(--text-muted)" }}>
            {task.id}
          </span>
          <button
            onClick={() => setSelectedTaskId(null)}
            className="text-xs px-2 py-0.5 rounded"
            style={{ background: "var(--bg-tertiary)", color: "var(--text-secondary)" }}
          >
            <X size={12} />
          </button>
        </div>

        <div
          className="text-xs font-semibold uppercase px-2 py-1 rounded inline-block"
          style={{ color: statusColor[task.status], background: "var(--bg-tertiary)" }}
        >
          {statusLabel[task.status] ?? task.status}
        </div>

        <div>
          <div className="text-xs mb-1" style={{ color: "var(--text-muted)" }}>描述</div>
          <div className="text-sm" style={{ color: "var(--text-primary)" }}>{task.description}</div>
        </div>

        <div>
          <div className="text-xs mb-1" style={{ color: "var(--text-muted)" }}>CLI 类型</div>
          <span
            className="text-xs px-2 py-0.5 rounded"
            style={{ background: "var(--bg-tertiary)", color: "var(--text-primary)" }}
          >
            {task.cli_type}
          </span>
        </div>

        <div>
          <div className="text-xs mb-1" style={{ color: "var(--text-muted)" }}>执行模式</div>
          <span className="text-xs" style={{ color: "var(--text-secondary)" }}>
            {task.execution_mode === "pipeline" ? "流水线" : "独立"}
          </span>
        </div>

        {task.depends_on.length > 0 && (
          <div>
            <div className="text-xs mb-1" style={{ color: "var(--text-muted)" }}>依赖任务</div>
            <div className="flex flex-wrap gap-1">
              {task.depends_on.map((dep) => (
                <button
                  key={dep}
                  onClick={() => setSelectedTaskId(dep)}
                  className="text-xs px-2 py-0.5 rounded cursor-pointer"
                  style={{ background: "var(--bg-tertiary)", color: "var(--accent)" }}
                >
                  {dep}
                </button>
              ))}
            </div>
          </div>
        )}

        <div>
          <div className="text-xs mb-1" style={{ color: "var(--text-muted)" }}>提示词</div>
          <div
            className="text-xs font-mono p-2 rounded overflow-auto"
            style={{
              background: "var(--bg-tertiary)",
              color: "var(--text-secondary)",
              maxHeight: 160,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {task.prompt}
          </div>
        </div>

        {task.output && (
          <div>
            <div className="text-xs mb-1" style={{ color: "var(--text-muted)" }}>输出</div>
            <div
              className="text-xs font-mono p-2 rounded overflow-auto"
              style={{
                background: "var(--bg-tertiary)",
                color: "var(--text-secondary)",
                maxHeight: 200,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
              }}
            >
              {task.output}
            </div>
          </div>
        )}

        {/* Chat history */}
        {chatMessages.length > 0 && (
          <div>
            <div className="text-xs mb-1" style={{ color: "var(--text-muted)" }}>对话</div>
            <div className="space-y-2">
              {chatMessages.map((msg, i) => (
                <div
                  key={i}
                  className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"} animate-fade-in`}
                >
                  <div
                    className="max-w-[90%] px-2 py-1.5 rounded text-xs whitespace-pre-wrap"
                    style={{
                      background: msg.role === "user" ? "var(--accent)" : "var(--bg-tertiary)",
                      color: msg.role === "user" ? "#fff" : "var(--text-primary)",
                      wordBreak: "break-word",
                      boxShadow: "var(--shadow-sm)",
                    }}
                  >
                    {msg.content}
                  </div>
                </div>
              ))}
              {isSending && (
                <div className="flex justify-start animate-fade-in">
                  <div
                    className="px-2 py-1.5 rounded text-xs flex items-center gap-1"
                    style={{ background: "var(--bg-tertiary)", color: "var(--text-muted)" }}
                  >
                    <span className="thinking-dot" style={{ animationDelay: "0ms" }} />
                    <span className="thinking-dot" style={{ animationDelay: "160ms" }} />
                    <span className="thinking-dot" style={{ animationDelay: "320ms" }} />
                  </div>
                </div>
              )}
              <div ref={chatEndRef} />
            </div>
          </div>
        )}
      </div>

      {/* Chat input (available when task has a session) */}
      {task.session_id && (
        <div
          className="flex gap-1.5 pt-2 mt-2 shrink-0"
          style={{ borderTop: "1px solid var(--border)" }}
        >
          <input
            value={chatInput}
            onChange={(e) => setChatInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSendMessage()}
            placeholder="追加指令..."
            disabled={isSending}
            className="flex-1 px-2 py-1.5 rounded text-xs outline-none"
            style={{
              background: "var(--bg-tertiary)",
              color: "var(--text-primary)",
              border: "1px solid var(--border)",
            }}
          />
          <button
            onClick={handleSendMessage}
            disabled={isSending || !chatInput.trim()}
            className="px-3 py-1.5 rounded text-xs disabled:opacity-50 flex items-center gap-1"
            style={{ background: "var(--accent)", color: "#fff" }}
          >
            <Send size={12} />
          </button>
        </div>
      )}
    </div>
  );
}
