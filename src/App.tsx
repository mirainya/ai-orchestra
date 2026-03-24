import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useThemeStore } from "./stores/theme";
import { useOrchestratorStore, type SubTask } from "./stores/orchestrator";
import { Layout } from "./components/Layout";
import { ToastContainer } from "./components/Toast";

export default function App() {
  const init = useThemeStore((s) => s.init);
  const {
    setTasks,
    updateTask,
    setWorkers,
    appendOutput,
    setPlanning,
    setRunning,
    appendPlannerMessage,
    setPlannerStatus,
    appendTaskMessage,
    addToast,
  } = useOrchestratorStore();

  useEffect(() => init(), [init]);

  useEffect(() => {
    let cancelled = false;
    const unlistenFns: (() => void)[] = [];

    async function setup() {
      const u1 = await listen<Record<string, unknown>>("task-update", (e) => {
        const d = e.payload;
        switch (d.type) {
          case "planning_start":
            setPlanning(true);
            setRunning(true);
            appendOutput("planner", `Planning: ${d.goal}`, "info");
            break;
          case "plan_ready": {
            setPlanning(false);
            const plan = d.plan as { tasks: SubTask[] };
            const tasks = plan.tasks.map((t) => ({ ...t, status: "pending" as const }));
            setTasks(tasks);
            appendOutput("planner", `Plan ready: ${tasks.length} tasks`, "success");
            addToast("info", `计划就绪：${tasks.length} 个任务`);
            break;
          }
          case "task_status": {
            const taskId = d.task_id as string;
            const status = d.status as SubTask["status"];
            updateTask(taskId, {
              status,
              output: d.output as string | undefined,
              session_id: d.session_id as string | undefined,
            });
            const level = status === "completed" ? "success" as const
              : status === "failed" ? "error" as const
              : "info" as const;
            appendOutput(taskId, `Status: ${status}`, level);
            if (status === "completed") {
              addToast("success", `任务 ${taskId} 已完成`);
            } else if (status === "failed") {
              addToast("error", `任务 ${taskId} 执行失败`);
            }
            break;
          }
          case "all_done":
            setRunning(false);
            appendOutput("system", "All tasks completed", "success");
            addToast("success", "所有任务已完成");
            break;
        }
      });
      if (cancelled) { u1(); return; }
      unlistenFns.push(u1);

      const u2 = await listen<Record<string, string>>("output-line", (e) => {
        const payload = e.payload;
        if (typeof payload === "string") {
          appendOutput("system", payload);
        } else {
          appendOutput(
            payload.source ?? "system",
            payload.text ?? String(payload),
            (payload.level as "info" | "error" | "success" | "warn") ?? "info"
          );
        }
      });
      if (cancelled) { u2(); unlistenFns.forEach((u) => u()); return; }
      unlistenFns.push(u2);

      const u3 = await listen<Record<string, string>>("worker-update", (e) => {
        const d = e.payload;
        const store = useOrchestratorStore.getState();
        const existing = store.workers;
        const updated = existing.map((w) =>
          w.name === d.name ? { ...w, status: d.status as "idle" | "busy" } : w
        );
        setWorkers(updated);
      });
      if (cancelled) { u3(); unlistenFns.forEach((u) => u()); return; }
      unlistenFns.push(u3);

      const u4 = await listen<Record<string, string>>("planner-message", (e) => {
        const d = e.payload;
        const status = d.status as "thinking" | "awaiting_approval";
        setPlannerStatus(status);
        if (d.content) {
          appendPlannerMessage({ role: "assistant", content: d.content });
        }
      });
      if (cancelled) { u4(); unlistenFns.forEach((u) => u()); return; }
      unlistenFns.push(u4);

      const u5 = await listen<Record<string, string>>("task-message", (e) => {
        const d = e.payload;
        if (d.task_id && d.content) {
          appendTaskMessage(d.task_id, {
            role: (d.role as "user" | "assistant") ?? "assistant",
            content: d.content,
          });
        }
      });
      if (cancelled) { u5(); unlistenFns.forEach((u) => u()); return; }
      unlistenFns.push(u5);
    }

    setup();

    return () => {
      cancelled = true;
      unlistenFns.forEach((u) => u());
    };
  }, [
    setTasks,
    updateTask,
    setWorkers,
    appendOutput,
    setPlanning,
    setRunning,
    appendPlannerMessage,
    setPlannerStatus,
    appendTaskMessage,
    addToast,
  ]);

  return (
    <>
      <Layout />
      <ToastContainer />
    </>
  );
}
