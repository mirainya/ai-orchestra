import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useOrchestratorStore } from "../stores/orchestrator";

export function PlannerChat() {
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const {
    plannerSessionId,
    plannerMessages,
    plannerStatus,
    isRunning,
    appendPlannerMessage,
    setPlannerStatus,
    setRunning,
    appendOutput,
  } = useOrchestratorStore();

  const busy = plannerStatus === "thinking" || isRunning;

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [plannerMessages]);

  if (!plannerSessionId) return null;

  const handleSend = async () => {
    const trimmed = input.trim();
    if (!trimmed || busy) return;
    setInput("");
    appendPlannerMessage({ role: "user", content: trimmed });
    setPlannerStatus("thinking");
    try {
      await invoke("send_planner_message", {
        sessionId: plannerSessionId,
        message: trimmed,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      appendOutput("planner", `Error: ${msg}`, "error");
      setPlannerStatus("awaiting_approval");
    }
  };

  const handleApprove = async () => {
    setPlannerStatus("thinking");
    setRunning(true);
    try {
      await invoke("approve_plan", { sessionId: plannerSessionId });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      appendOutput("planner", `Approve failed: ${msg}`, "error");
    } finally {
      setPlannerStatus("idle");
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Messages area */}
      <div
        className="flex-1 overflow-y-auto space-y-3 p-3"
        style={{ minHeight: 0 }}
      >
        {plannerMessages.map((msg, i) => (
          <div
            key={i}
            className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className="max-w-[80%] px-3 py-2 rounded-lg text-sm whitespace-pre-wrap"
              style={{
                background:
                  msg.role === "user" ? "var(--accent)" : "var(--bg-tertiary)",
                color: msg.role === "user" ? "#fff" : "var(--text-primary)",
              }}
            >
              {msg.content}
            </div>
          </div>
        ))}
        {plannerStatus === "thinking" && (
          <div className="flex justify-start">
            <div
              className="px-3 py-2 rounded-lg text-sm"
              style={{
                background: "var(--bg-tertiary)",
                color: "var(--text-muted)",
              }}
            >
              思考中...
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Action bar */}
      {plannerStatus === "awaiting_approval" && !isRunning && (
        <div
          className="flex items-center gap-2 px-3 py-2"
          style={{ borderTop: "1px solid var(--border)" }}
        >
          <button
            onClick={handleApprove}
            className="px-4 py-2 rounded-lg font-medium text-sm"
            style={{ background: "var(--success)", color: "#fff" }}
          >
            批准计划
          </button>
          <span
            className="text-xs"
            style={{ color: "var(--text-muted)" }}
          >
            或在下方输入修改意见
          </span>
        </div>
      )}

      {/* Input */}
      <div
        className="flex gap-2 p-3"
        style={{ borderTop: "1px solid var(--border)" }}
      >
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSend()}
          placeholder={
            isRunning
              ? "执行中..."
              : "输入修改意见或追加指令..."
          }
          disabled={busy}
          className="flex-1 px-3 py-2 rounded-lg outline-none text-sm"
          style={{
            background: "var(--bg-tertiary)",
            color: "var(--text-primary)",
            border: "1px solid var(--border)",
          }}
        />
        <button
          onClick={handleSend}
          disabled={busy || !input.trim()}
          className="px-4 py-2 rounded-lg text-sm disabled:opacity-50"
          style={{ background: "var(--accent)", color: "#fff" }}
        >
          发送
        </button>
      </div>
    </div>
  );
}
