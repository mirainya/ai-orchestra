import { useState } from "react";
import { ThemeToggle } from "./ThemeToggle";
import { TaskInput } from "./TaskInput";
import { DagView } from "./DagView";
import { WorkerPanel } from "./WorkerPanel";
import { OutputPanel } from "./OutputPanel";
import { ConfigPanel } from "./ConfigPanel";
import { TaskDetail } from "./TaskDetail";
import { StatusBar } from "./StatusBar";
import { HistoryPanel } from "./HistoryPanel";
import { SettingsPanel } from "./SettingsPanel";
import { PlannerChat } from "./PlannerChat";
import { useOrchestratorStore } from "../stores/orchestrator";

type SidebarTab = "workers" | "config" | "history" | "detail";

export function Layout() {
  const [sidebarTab, setSidebarTab] = useState<SidebarTab>("workers");
  const [showSettings, setShowSettings] = useState(false);
  const selectedTaskId = useOrchestratorStore((s) => s.selectedTaskId);
  const plannerSessionId = useOrchestratorStore((s) => s.plannerSessionId);

  const activeTab = selectedTaskId ? "detail" : sidebarTab;

  return (
    <div
      className="h-screen flex flex-col"
      style={{ background: "var(--bg-primary)" }}
    >
      {/* Header */}
      <header
        className="flex items-center justify-between px-5 py-3 shrink-0"
        style={{ borderBottom: "1px solid var(--border)" }}
      >
        <div className="flex items-center gap-3">
          <div
            className="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold"
            style={{ background: "var(--accent)", color: "#fff" }}
          >
            AI
          </div>
          <div>
            <h1 className="text-sm font-bold" style={{ color: "var(--text-primary)" }}>
              AI CLI 编排器
            </h1>
            <p style={{ fontSize: 11, color: "var(--text-muted)" }}>
              多智能体任务编排
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowSettings(true)}
            className="p-2 rounded-lg transition-colors"
            style={{ background: "var(--bg-tertiary)", color: "var(--text-primary)" }}
            title="设置"
          >
            {"\u2699"}
          </button>
          <ThemeToggle />
        </div>
      </header>

      {/* Task Input */}
      <div className="px-5 py-3 shrink-0" style={{ borderBottom: "1px solid var(--border)" }}>
        <TaskInput />
      </div>

      {/* Main Area */}
      <div className="flex-1 flex min-h-0">
        {/* Left: DAG + Planner Chat */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* DAG View */}
          <div className={plannerSessionId ? "flex-1 min-h-0" : "flex-1"}>
            <DagView />
          </div>
          {/* Planner Chat (when active) */}
          {plannerSessionId && (
            <div
              className="shrink-0"
              style={{
                height: 280,
                borderTop: "1px solid var(--border)",
              }}
            >
              <PlannerChat />
            </div>
          )}
        </div>

        {/* Right Sidebar */}
        <div
          className="w-72 flex flex-col shrink-0"
          style={{ borderLeft: "1px solid var(--border)" }}
        >
          {/* Sidebar Tabs */}
          <div
            className="flex shrink-0"
            style={{ borderBottom: "1px solid var(--border)" }}
          >
            {([
              { key: "workers" as const, label: "工作者" },
              { key: "config" as const, label: "配置" },
              { key: "history" as const, label: "历史" },
            ]).map((tab) => (
              <button
                key={tab.key}
                onClick={() => {
                  setSidebarTab(tab.key);
                  if (selectedTaskId) {
                    useOrchestratorStore.getState().setSelectedTaskId(null);
                  }
                }}
                className="flex-1 py-2 text-xs font-medium transition-colors"
                style={{
                  color: activeTab === tab.key ? "var(--accent)" : "var(--text-muted)",
                  borderBottom: activeTab === tab.key ? "2px solid var(--accent)" : "2px solid transparent",
                  background: "transparent",
                }}
              >
                {tab.label}
              </button>
            ))}
            {selectedTaskId && (
              <button
                className="flex-1 py-2 text-xs font-medium"
                style={{
                  color: "var(--accent)",
                  borderBottom: "2px solid var(--accent)",
                  background: "transparent",
                }}
              >
                任务详情
              </button>
            )}
          </div>

          {/* Sidebar Content */}
          <div className="flex-1 overflow-y-auto p-3">
            {activeTab === "workers" && <WorkerPanel />}
            {activeTab === "config" && <ConfigPanel />}
            {activeTab === "history" && <HistoryPanel />}
            {activeTab === "detail" && selectedTaskId && <TaskDetail />}
          </div>
        </div>
      </div>

      {/* Output */}
      <div style={{ borderTop: "1px solid var(--border)" }}>
        <OutputPanel />
      </div>

      {/* Status Bar */}
      <StatusBar />

      {/* Settings Modal */}
      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
    </div>
  );
}
