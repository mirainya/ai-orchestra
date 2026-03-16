#[cfg(test)]
mod integration_tests {
    use crate::planner::parser::parse_plan;
    use crate::planner::validate_plan;
    use crate::config::{WorkerConfig, WorkerMode, WorkerRole};

    /// Real Claude CLI output (--output-format json) from functional test
    const CLAUDE_RAW_OUTPUT: &str = r#"{"type":"result","subtype":"success","is_error":false,"result":"{\n  \"goal\": \"写一个 Python 脚本实现简单的 TODO 应用（命令行版），并为它写一份中文使用说明文档\",\n  \"tasks\": [\n    {\n      \"id\": \"task-1\",\n      \"description\": \"实现 Python 命令行 TODO 应用脚本\",\n      \"cli_type\": \"claude_cli\",\n      \"depends_on\": [],\n      \"prompt\": \"请用 Python 编写一个命令行 TODO 应用脚本\",\n      \"execution_mode\": \"independent\"\n    },\n    {\n      \"id\": \"task-2\",\n      \"description\": \"编写 TODO 应用的中文使用说明文档\",\n      \"cli_type\": \"claude_cli\",\n      \"depends_on\": [\"task-1\"],\n      \"prompt\": \"请为 todo.py 编写中文使用说明文档\",\n      \"execution_mode\": \"pipeline\"\n    }\n  ]\n}"}"#;

    fn make_executor(name: &str, cli_type: &str, skills: Vec<&str>) -> WorkerConfig {
        WorkerConfig {
            name: name.to_string(),
            cli_type: cli_type.to_string(),
            mode: WorkerMode::Cli,
            role: WorkerRole::Executor,
            skills: skills.into_iter().map(String::from).collect(),
            cli_path: Some("claude".to_string()),
            extra_args: vec![],
            api_base_url: None,
            api_key: None,
            model: None,
        }
    }

    #[test]
    fn test_parse_real_claude_output() {
        let plan = parse_plan(CLAUDE_RAW_OUTPUT).unwrap();
        assert_eq!(plan.tasks.len(), 2);
        assert_eq!(plan.tasks[0].id, "task-1");
        assert_eq!(plan.tasks[1].id, "task-2");
        // Both should be claude_cli
        for task in &plan.tasks {
            let cli_str = serde_json::to_value(&task.cli_type).unwrap();
            assert_eq!(cli_str.as_str().unwrap(), "claude_cli");
        }
        // task-2 depends on task-1
        assert!(plan.tasks[1].depends_on.contains(&"task-1".to_string()));
        println!("Plan parsed OK: goal={}, tasks={}", plan.goal, plan.tasks.len());
    }

    #[test]
    fn test_validate_plan_pass() {
        let plan = parse_plan(CLAUDE_RAW_OUTPUT).unwrap();
        let e1 = make_executor("claude-coder", "claude_cli", vec!["coding"]);
        let e2 = make_executor("claude-writer", "claude_cli", vec!["writing"]);
        let executors: Vec<&WorkerConfig> = vec![&e1, &e2];
        // Should pass: all tasks use claude_cli which is in executors
        assert!(validate_plan(&plan, &executors).is_ok());
    }

    #[test]
    fn test_validate_plan_fail_missing_type() {
        let plan = parse_plan(CLAUDE_RAW_OUTPUT).unwrap();
        // Only provide openai executor, no claude_cli
        let e1 = make_executor("gpt-worker", "openai", vec!["general"]);
        let executors: Vec<&WorkerConfig> = vec![&e1];
        // Should fail: tasks use claude_cli but only openai is available
        let result = validate_plan(&plan, &executors);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("claude_cli"));
        println!("Validation correctly rejected: {}", err_msg);
    }

    #[test]
    fn test_build_prompt_dynamic() {
        let e1 = make_executor("claude-coder", "claude_cli", vec!["代码生成", "重构"]);
        let e2 = make_executor("claude-writer", "claude_cli", vec!["文档撰写", "翻译"]);
        let executors: Vec<&WorkerConfig> = vec![&e1, &e2];
        let prompt = crate::planner::build_planner_system_prompt(&executors);
        // Should contain worker names
        assert!(prompt.contains("claude-coder"));
        assert!(prompt.contains("claude-writer"));
        // Should contain skills
        assert!(prompt.contains("代码生成"));
        assert!(prompt.contains("文档撰写"));
        // Should contain cli_type enum constraint
        assert!(prompt.contains("\"claude_cli\""));
        // Should NOT contain other types
        assert!(!prompt.contains("\"codex_cli\""));
        assert!(!prompt.contains("\"openai\""));
        println!("Dynamic prompt length: {} chars", prompt.len());
    }
}
