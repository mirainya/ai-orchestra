import { useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  MarkerType,
  Position,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useOrchestratorStore, type SubTask, type TaskStatus } from "../stores/orchestrator";

const statusColor: Record<TaskStatus, string> = {
  pending: "var(--text-muted)",
  running: "var(--warning)",
  completed: "var(--success)",
  failed: "var(--error)",
};

const statusIcon: Record<TaskStatus, string> = {
  pending: "\u25cb",
  running: "\u25d4",
  completed: "\u2714",
  failed: "\u2718",
};

const NODE_W = 200;
const GAP_X = 260;
const GAP_Y = 120;

/** Compute layered layout using topological levels */
function computeLayout(tasks: SubTask[]): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();
  if (tasks.length === 0) return positions;

  const inDegree = new Map<string, number>();
  const dependents = new Map<string, string[]>();

  for (const t of tasks) {
    inDegree.set(t.id, 0);
  }
  for (const t of tasks) {
    for (const dep of t.depends_on) {
      inDegree.set(t.id, (inDegree.get(t.id) ?? 0) + 1);
      const d = dependents.get(dep) ?? [];
      d.push(t.id);
      dependents.set(dep, d);
    }
  }

  // BFS by levels (Kahn's algorithm)
  const levels: string[][] = [];
  let queue = tasks.filter((t) => (inDegree.get(t.id) ?? 0) === 0).map((t) => t.id);
  const visited = new Set<string>();

  while (queue.length > 0) {
    const level: string[] = [];
    const next: string[] = [];
    for (const id of queue) {
      if (visited.has(id)) continue;
      visited.add(id);
      level.push(id);
      for (const dep of dependents.get(id) ?? []) {
        const deg = (inDegree.get(dep) ?? 1) - 1;
        inDegree.set(dep, deg);
        if (deg === 0) next.push(dep);
      }
    }
    if (level.length > 0) levels.push(level);
    queue = next;
  }

  // Assign positions: each level is a row, centered horizontally
  const maxWidth = Math.max(...levels.map((l) => l.length));
  for (let row = 0; row < levels.length; row++) {
    const level = levels[row];
    const totalWidth = level.length * GAP_X;
    const offsetX = (maxWidth * GAP_X - totalWidth) / 2;
    for (let col = 0; col < level.length; col++) {
      positions.set(level[col], {
        x: offsetX + col * GAP_X,
        y: row * GAP_Y,
      });
    }
  }

  return positions;
}

export function DagView() {
  const tasks = useOrchestratorStore((s) => s.tasks);
  const isPlanning = useOrchestratorStore((s) => s.isPlanning);

  const { nodes, edges } = useMemo(() => {
    const positions = computeLayout(tasks);

    const nodes: Node[] = tasks.map((t) => {
      const pos = positions.get(t.id) ?? { x: 0, y: 0 };
      return {
        id: t.id,
        position: pos,
        sourcePosition: Position.Bottom,
        targetPosition: Position.Top,
        data: {
          label: (
            <div style={{ padding: "6px 8px", minWidth: 160 }}>
              <div className="flex items-center gap-1.5" style={{ marginBottom: 4 }}>
                <span style={{ color: statusColor[t.status], fontSize: 13 }}>
                  {statusIcon[t.status]}
                </span>
                <span
                  style={{
                    fontSize: 10,
                    color: statusColor[t.status],
                    fontWeight: 700,
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                  }}
                >
                  {t.status}
                </span>
                <span
                  style={{
                    fontSize: 9,
                    color: "var(--text-muted)",
                    marginLeft: "auto",
                    background: "var(--bg-tertiary)",
                    padding: "1px 5px",
                    borderRadius: 4,
                  }}
                >
                  {t.cli_type}
                </span>
              </div>
              <div
                style={{
                  fontSize: 12,
                  lineHeight: 1.3,
                  color: "var(--text-primary)",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  display: "-webkit-box",
                  WebkitLineClamp: 2,
                  WebkitBoxOrient: "vertical",
                }}
              >
                {t.description}
              </div>
              <div
                style={{
                  fontSize: 9,
                  color: "var(--text-muted)",
                  marginTop: 3,
                  fontFamily: "monospace",
                }}
              >
                {t.id}
              </div>
            </div>
          ),
        },
        style: {
          background: "var(--node-bg)",
          border: `2px solid ${statusColor[t.status]}`,
          borderRadius: 10,
          color: "var(--text-primary)",
          width: NODE_W,
          boxShadow:
            t.status === "running"
              ? `0 0 12px ${statusColor[t.status]}40`
              : "0 1px 3px rgba(0,0,0,0.2)",
          transition: "border-color 0.3s, box-shadow 0.3s",
        },
      };
    });

    const edges: Edge[] = tasks.flatMap((t) =>
      t.depends_on.map((dep) => ({
        id: `${dep}->${t.id}`,
        source: dep,
        target: t.id,
        animated: t.status === "running",
        type: "smoothstep",
        markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
        style: {
          stroke:
            t.status === "running"
              ? "var(--warning)"
              : t.status === "completed"
                ? "var(--success)"
                : "var(--border)",
          strokeWidth: 2,
        },
      }))
    );

    return { nodes, edges };
  }, [tasks]);

  if (tasks.length === 0) {
    return (
      <div
        className="flex-1 flex flex-col items-center justify-center rounded-xl gap-3"
        style={{ background: "var(--bg-secondary)", color: "var(--text-muted)" }}
      >
        {isPlanning ? (
          <>
            <div className="animate-spin" style={{ fontSize: 24 }}>{"\u2699"}</div>
            <div style={{ fontSize: 14 }}>正在规划任务...</div>
          </>
        ) : (
          <>
            <div style={{ fontSize: 32, opacity: 0.3 }}>{"\u25c7"}</div>
            <div style={{ fontSize: 14 }}>在上方输入任务以生成执行 DAG</div>
          </>
        )}
      </div>
    );
  }

  return (
    <div
      className="flex-1 rounded-xl overflow-hidden"
      style={{ background: "var(--bg-secondary)" }}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        proOptions={{ hideAttribution: true }}
        nodesDraggable
        nodesConnectable={false}
      >
        <Background gap={20} size={1} />
        <Controls
          showInteractive={false}
          style={{ background: "var(--bg-tertiary)", borderColor: "var(--border)" }}
        />
        <MiniMap
          nodeColor={(n) => {
            const task = tasks.find((t) => t.id === n.id);
            if (!task) return "var(--text-muted)";
            return statusColor[task.status] ?? "var(--text-muted)";
          }}
          style={{ background: "var(--bg-tertiary)" }}
          maskColor="rgba(0,0,0,0.3)"
        />
      </ReactFlow>
    </div>
  );
}
