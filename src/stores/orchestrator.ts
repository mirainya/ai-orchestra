import { create } from "zustand";

export type TaskStatus = "pending" | "running" | "completed" | "failed";
export type CliType = "claude" | "codex" | "glm" | "claude_cli" | "codex_cli" | "glm_cli" | "openai" | "anthropic";

export interface SubTask {
  id: string;
  description: string;
  cli_type: CliType;
  depends_on: string[];
  prompt: string;
  status: TaskStatus;
  output?: string;
  execution_mode?: "independent" | "pipeline";
  session_id?: string;
}

export interface WorkerInfo {
  name: string;
  cli_type: string;
  status: "idle" | "busy";
  role?: string;
  skills?: string[];
  current_task?: string;
}

export interface OutputLine {
  timestamp: number;
  source: string; // "planner" | "system" | task id
  level: "info" | "error" | "success" | "warn";
  text: string;
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export type PlannerStatus = "idle" | "thinking" | "awaiting_approval";

interface OrchestratorState {
  goal: string;
  tasks: SubTask[];
  workers: WorkerInfo[];
  outputLines: OutputLine[];
  isPlanning: boolean;
  isRunning: boolean;
  selectedTaskId: string | null;
  workingDir: string | null;

  // Planner chat state
  plannerSessionId: string | null;
  plannerMessages: ChatMessage[];
  plannerStatus: PlannerStatus;

  // Task chat state
  taskSessions: Record<string, ChatMessage[]>;

  setGoal: (goal: string) => void;
  setTasks: (tasks: SubTask[]) => void;
  updateTask: (id: string, patch: Partial<SubTask>) => void;
  setWorkers: (workers: WorkerInfo[]) => void;
  appendOutput: (source: string, text: string, level?: OutputLine["level"]) => void;
  setPlanning: (v: boolean) => void;
  setRunning: (v: boolean) => void;
  setSelectedTaskId: (id: string | null) => void;
  setWorkingDir: (dir: string | null) => void;

  // Planner chat actions
  setPlannerSessionId: (id: string | null) => void;
  appendPlannerMessage: (msg: ChatMessage) => void;
  setPlannerStatus: (status: PlannerStatus) => void;

  // Task chat actions
  appendTaskMessage: (taskId: string, msg: ChatMessage) => void;

  reset: () => void;
  clearOutput: () => void;
}

export const useOrchestratorStore = create<OrchestratorState>((set) => ({
  goal: "",
  tasks: [],
  workers: [],
  outputLines: [],
  isPlanning: false,
  isRunning: false,
  selectedTaskId: null,
  workingDir: null,
  plannerSessionId: null,
  plannerMessages: [],
  plannerStatus: "idle",
  taskSessions: {},

  setGoal: (goal) => set({ goal }),
  setTasks: (tasks) => set({ tasks }),
  updateTask: (id, patch) =>
    set((s) => ({
      tasks: s.tasks.map((t) => (t.id === id ? { ...t, ...patch } : t)),
    })),
  setWorkers: (workers) => set({ workers }),
  appendOutput: (source, text, level = "info") =>
    set((s) => ({
      outputLines: [
        ...s.outputLines,
        { timestamp: Date.now(), source, level, text },
      ],
    })),
  setPlanning: (isPlanning) => set({ isPlanning }),
  setRunning: (isRunning) => set({ isRunning }),
  setSelectedTaskId: (selectedTaskId) => set({ selectedTaskId }),
  setWorkingDir: (workingDir) => set({ workingDir }),

  setPlannerSessionId: (plannerSessionId) => set({ plannerSessionId }),
  appendPlannerMessage: (msg) =>
    set((s) => ({
      plannerMessages: [...s.plannerMessages, msg],
    })),
  setPlannerStatus: (plannerStatus) => set({ plannerStatus }),

  appendTaskMessage: (taskId, msg) =>
    set((s) => ({
      taskSessions: {
        ...s.taskSessions,
        [taskId]: [...(s.taskSessions[taskId] ?? []), msg],
      },
    })),

  reset: () =>
    set({
      goal: "",
      tasks: [],
      outputLines: [],
      isPlanning: false,
      isRunning: false,
      selectedTaskId: null,
      plannerSessionId: null,
      plannerMessages: [],
      plannerStatus: "idle",
      taskSessions: {},
    }),
  clearOutput: () => set({ outputLines: [] }),
}));
