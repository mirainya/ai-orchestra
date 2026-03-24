import { useCallback, useState } from "react";
import { Settings, Users, Sliders, History, FileText } from "lucide-react";
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
import { useResizable } from "../hooks/useResizable";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";

type SidebarTab = "workers" | "config" | "history" | "detail";

function ResizeHandle({ direction, ...props }: { direction: "vertical" | "horizontal" } & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      {...props}
      style={{
        ...props.style,
        [direction === "vertical" ? "height" : "width"]: 6,
        cursor: direction === "vertical" ? "row-resize" : "col-resize",
        position: "relative",
        zIndex: 5,
        flexShrink: 0,
      }}
    >
      <div
        style={{
          position: "absolute",
          [direction === "vertical" ? "left" : "top"]: "50%",
          [direction === "vertical" ? "top" : "left"]: "50%",
          transform: "translate(-50%, -50%)",
          [direction === "vertical" ? "width" : "height"]: 32,
          [direction === "vertical" ? "height" : "width"]: 3,
          borderRadius: 2,
          background: "var(--border)",
          transition: "background var(--transition-fast)",
        }}
      />
    </div>
  );
}

export function Layout() {
  const [sidebarTab, setSidebarTab] = useState<SidebarTab>("workers");
  const [showSettings, setShowSettings] = useState(false);
  const selectedTaskId = useOrchestratorStore((s) => s.selectedTaskId);
  const plannerSessionId = useOrchestratorStore((s) => s.plannerSessionId);

  const outputResize = useResizable({ direction: "vertical", initialSize: 200, minSize: 120, maxSize: 400 });
  const plannerResize = useResizable({ direction: "vertical", initialSize: 280, minSize: 150, maxSize: 500 });

  const activeTab = selectedTaskId ? "detail" : sidebarTab;

  useKeyboardShortcuts({
    onCloseSettings: useCallback(() => setShowSettings(false), []),
  });

  return (
    <div
      className="h-screen flex flex-col"
      style={{ background: "var(--bg-primary)" }}
    >
      {/* Header */}
      <header
        className="flex items-center justify-between px-5 py-3 shrink-0"
        style={{
          borderBottom: "1px solid var(--border)",
          boxShadow: "var(--shadow-sm)",
          position: "relative",
          zIndex: 10,
        }}
      >
        <div className="flex items-center gap-3">
          <div
            className="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold"
            style={{
              background: "linear-gradient(135deg, var(--accent), var(--accent-hover))",
              color: "#fff",
            }}
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
            <Settings size={16} />
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
        <div className="flex-1 flex flex-col min-w-0 min-h-0">
          {/* DAG View */}
          <div className={plannerSessionId ? "flex-1 min-h-0 overflow-hidden" : "flex-1"}>
            <DagView />
          </div>
          {/* Planner Chat (when active) */}
          {plannerSessionId && (
            <>
              <ResizeHandle direction="vertical" {...plannerResize.handleProps} />
              <div
                className="shrink-0"
                style={{
                  height: plannerResize.size,
                  borderTop: "1px solid var(--border)",
                }}
              >
                <PlannerChat />
              </div>
            </>
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
              { key: "workers" as const, label: "工作者", icon: <Users size={14} /> },
              { key: "config" as const, label: "配置", icon: <Sliders size={14} /> },
              { key: "history" as const, label: "历史", icon: <History size={14} /> },
            ]).map((tab) => (
              <button
                key={tab.key}
                onClick={() => {
                  setSidebarTab(tab.key);
                  if (selectedTaskId) {
                    useOrchestratorStore.getState().setSelectedTaskId(null);
                  }
                }}
                className="flex-1 py-2 text-xs font-medium transition-colors flex items-center justify-center gap-1.5"
                style={{
                  color: activeTab === tab.key ? "var(--accent)" : "var(--text-muted)",
                  borderBottom: activeTab === tab.key ? "2px solid var(--accent)" : "2px solid transparent",
                  background: "transparent",
                }}
              >
                {tab.icon}
                {tab.label}
              </button>
            ))}
            {selectedTaskId && (
              <button
                className="flex-1 py-2 text-xs font-medium flex items-center justify-center gap-1.5"
                style={{
                  color: "var(--accent)",
                  borderBottom: "2px solid var(--accent)",
                  background: "transparent",
                }}
              >
                <FileText size={14} />
                任务详情
              </button>
            )}
          </div>

          {/* Sidebar Content */}
          <div className="flex-1 overflow-y-auto p-3 animate-fade-in">
            {activeTab === "workers" && <WorkerPanel />}
            {activeTab === "config" && <ConfigPanel />}
            {activeTab === "history" && <HistoryPanel />}
            {activeTab === "detail" && selectedTaskId && <TaskDetail />}
          </div>
        </div>
      </div>

      {/* Output */}
      <ResizeHandle direction="vertical" {...outputResize.handleProps} />
      <div style={{ borderTop: "1px solid var(--border)", height: outputResize.size }}>
        <OutputPanel />
      </div>

      {/* Status Bar */}
      <StatusBar />

      {/* Settings Drawer */}
      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
    </div>
  );
}
