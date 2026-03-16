use super::{Plan, SubTask};
use std::collections::{HashMap, HashSet, VecDeque};

/// Returns task IDs in topological order, grouped by execution level (parallelizable).
pub fn schedule(plan: &Plan) -> anyhow::Result<Vec<Vec<String>>> {
    let task_map: HashMap<&str, &SubTask> = plan.tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    // Compute in-degree
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for t in &plan.tasks {
        in_degree.entry(t.id.as_str()).or_insert(0);
        for dep in &t.depends_on {
            if !task_map.contains_key(dep.as_str()) {
                anyhow::bail!("Task {} depends on unknown task {}", t.id, dep);
            }
            *in_degree.entry(t.id.as_str()).or_insert(0) += 1;
            dependents.entry(dep.as_str()).or_default().push(t.id.as_str());
        }
    }

    let mut levels: Vec<Vec<String>> = Vec::new();
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut visited: HashSet<&str> = HashSet::new();

    while !queue.is_empty() {
        let mut level = Vec::new();
        let mut next_queue = VecDeque::new();

        while let Some(id) = queue.pop_front() {
            if visited.contains(id) {
                continue;
            }
            visited.insert(id);
            level.push(id.to_string());

            if let Some(deps) = dependents.get(id) {
                for &dep_id in deps {
                    let deg = in_degree.get_mut(dep_id).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next_queue.push_back(dep_id);
                    }
                }
            }
        }

        if !level.is_empty() {
            levels.push(level);
        }
        queue = next_queue;
    }

    if visited.len() != plan.tasks.len() {
        anyhow::bail!("Cycle detected in task dependencies");
    }

    Ok(levels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{CliType, ExecMode};

    fn make_task(id: &str, deps: Vec<&str>) -> SubTask {
        SubTask {
            id: id.to_string(),
            description: id.to_string(),
            cli_type: CliType::Codex,
            depends_on: deps.into_iter().map(String::from).collect(),
            prompt: String::new(),
            execution_mode: ExecMode::Independent,
        }
    }

    #[test]
    fn test_linear() {
        let plan = Plan {
            goal: "test".into(),
            tasks: vec![
                make_task("a", vec![]),
                make_task("b", vec!["a"]),
                make_task("c", vec!["b"]),
            ],
        };
        let levels = schedule(&plan).unwrap();
        assert_eq!(levels.len(), 3);
    }

    #[test]
    fn test_parallel() {
        let plan = Plan {
            goal: "test".into(),
            tasks: vec![
                make_task("a", vec![]),
                make_task("b", vec![]),
                make_task("c", vec!["a", "b"]),
            ],
        };
        let levels = schedule(&plan).unwrap();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let plan = Plan {
            goal: "test".into(),
            tasks: vec![
                make_task("a", vec!["b"]),
                make_task("b", vec!["a"]),
            ],
        };
        assert!(schedule(&plan).is_err());
    }
}
