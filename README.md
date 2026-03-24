# AI CLI Controller

一个基于 Tauri 2 + React + Rust 的桌面端多智能体任务编排器，用来把一个复杂目标拆解成多个可并行或串行执行的子任务，并交给不同的 AI CLI / API Worker 执行。

目前项目重点能力包括：规划器自动拆解任务、DAG 可视化、执行器调度、多轮对话、运行历史记录，以及可配置的多种 Worker 适配。

## 功能特性

- 多 Worker 编排：支持 Planner / Executor 角色分离
- 任务拆解：Planner 根据目标自动生成任务 DAG
- DAG 调度执行：按依赖关系分层调度，支持并行执行
- 多轮对话：支持 Planner 对话和 Task 对话追加消息
- 多种适配器：已包含 Claude、Codex、GLM 以及 OpenAI / Anthropic API 相关适配
- 工作目录支持：执行任务时可传入指定 working directory
- 历史记录：保存每次运行的计划、结果与状态
- 桌面端 UI：包含任务输入、DAG 视图、输出面板、Worker 面板、配置面板、历史记录、任务详情
- 体验增强：主题切换、Toast 通知、可拖拽面板、设置抽屉、快捷键

## 技术栈

### 前端

- React 19
- TypeScript
- Zustand
- Vite 6
- Tailwind CSS 4
- Tauri API
- @xyflow/react（DAG 可视化）
- lucide-react

### 后端

- Rust 2021
- Tauri 2
- Tokio
- Serde / serde_json
- Reqwest

## 项目结构

```text
.
├─ src/                     # React 前端
│  ├─ components/           # UI 组件
│  ├─ stores/               # Zustand 状态管理
│  └─ hooks/                # 自定义 hooks
├─ src-tauri/
│  ├─ src/
│  │  ├─ commands.rs        # Tauri IPC 命令入口
│  │  ├─ planner/           # 规划器逻辑与解析
│  │  ├─ worker/            # Worker 抽象、适配器、调度执行
│  │  ├─ dag/               # Plan / SubTask / 调度器
│  │  ├─ history/           # 历史记录
│  │  └─ session/           # 多轮会话管理
├─ config.toml              # Worker 与执行配置
└─ README.md
```

## 核心流程

1. 用户在前端输入目标，并可选择工作目录。
2. 后端找到 `planner` 角色的 Worker，生成 JSON 形式的任务计划。
3. 计划被解析为 DAG，并校验 `cli_type` 是否能被现有执行器处理。
4. 调度器按依赖关系分层执行任务，同层任务可并行运行。
5. 每个任务会创建独立 session，支持后续追加对话。
6. 执行结果实时推送到前端，并写入历史记录。

## 当前已实现的关键能力

- 自动规划任务并返回 `Plan`
- `pipeline` / `independent` 两种执行模式
- Worker 池状态管理（idle / busy）
- CLI 模式与 API 模式双通路
- Planner 多轮对话
- Task 执行后继续追问
- 运行历史查看与删除
- `config.toml` 多位置查找，方便开发与打包场景

## 配置说明

项目使用根目录 `config.toml` 配置 Worker。

示例配置中可以看到：

- `role = "planner"` 的 Worker 用于生成计划
- `role = "executor"` 的 Worker 用于执行子任务
- `mode = "cli"` 或 `mode = "api"` 表示执行方式
- `cli_path` / `extra_args` 用于 CLI Worker
- `task_timeout_secs`、`max_retries`、`planner_timeout_secs` 用于执行控制

示例片段：

```toml
[[workers]]
name = "claude-planner"
cli_type = "claude_cli"
mode = "cli"
role = "planner"
cli_path = "claude"
extra_args = ["--output-format", "json"]

[[workers]]
name = "codex-1"
cli_type = "claude_cli"
mode = "cli"
role = "executor"
cli_path = "codex"
extra_args = ["--approval-mode", "full-auto", "-q"]

[execution]
task_timeout_secs = 300
max_retries = 1
planner_timeout_secs = 120
```

## 开发环境要求

建议准备以下环境：

- Node.js 18+
- Rust stable
- Tauri 2 开发环境
- 对应的 AI CLI 工具（如 `claude`、`codex`、`glm`）已安装并可在命令行直接调用

> Windows 下开发 Tauri 时，还需要满足 Tauri 官方要求的 WebView2 / Visual Studio C++ Build Tools 等依赖。

## 安装依赖

```bash
npm install
```

## 启动开发环境

```bash
npm run tauri dev
```

如果只想调试前端，也可以使用：

```bash
npm run dev
```

## 构建

```bash
npm run tauri build
```

## 使用方式

1. 启动应用。
2. 在顶部输入任务目标。
3. 选择可选的工作目录。
4. 提交后等待 Planner 生成计划。
5. 在 DAG 中观察任务依赖与状态变化。
6. 在输出面板查看日志。
7. 在任务详情中继续和某个执行任务对话。
8. 在历史记录中查看过去运行结果。

## 关键文件

- `src/App.tsx`：前端事件监听与全局状态更新
- `src/components/Layout.tsx`：主界面布局
- `src/stores/orchestrator.ts`：前端核心状态
- `src-tauri/src/lib.rs`：Tauri 启动入口与命令注册
- `src-tauri/src/commands.rs`：任务提交、规划、审批、会话消息、历史记录
- `src-tauri/src/planner/mod.rs`：规划 prompt 构建、CLI/API 规划生成、计划校验
- `src-tauri/src/worker/mod.rs`：CLI 执行与输出流处理

## 后续可补充方向

- 更完善的 README 截图与交互演示
- 支持更多 AI Worker 的能力声明与路由策略
- 更细粒度的任务失败恢复与人工介入
- 计划编辑与可视化调整
- 会话持久化与项目级配置管理

## License

暂未指定。
