use anyhow::Result;
use crate::dag::Plan;

/// Try multiple strategies to extract a Plan from CLI output.
pub fn parse_plan(raw: &str) -> Result<Plan> {
    // Strategy 1: direct JSON parse
    if let Ok(plan) = serde_json::from_str::<Plan>(raw) {
        return Ok(plan);
    }

    // Strategy 2: extract from Claude JSON output format (has "result" field)
    if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(result) = wrapper.get("result").and_then(|r| r.as_str()) {
            if let Ok(plan) = parse_plan(result) {
                return Ok(plan);
            }
        }
    }

    // Strategy 3: extract JSON from markdown code block
    if let Some(start) = raw.find("```json") {
        let after = &raw[start + 7..];
        if let Some(end) = after.find("```") {
            let json_str = after[..end].trim();
            if let Ok(plan) = serde_json::from_str::<Plan>(json_str) {
                return Ok(plan);
            }
        }
    }

    // Strategy 4: find first { ... } block
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            let json_str = &raw[start..=end];
            if let Ok(plan) = serde_json::from_str::<Plan>(json_str) {
                return Ok(plan);
            }
        }
    }

    anyhow::bail!("Failed to parse plan from CLI output:\n{}", &raw[..raw.len().min(500)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_json() {
        let json = r#"{"goal":"test","tasks":[{"id":"t1","description":"do thing","cli_type":"codex","depends_on":[],"prompt":"do it"}]}"#;
        let plan = parse_plan(json).unwrap();
        assert_eq!(plan.goal, "test");
        assert_eq!(plan.tasks.len(), 1);
    }

    #[test]
    fn test_code_block() {
        let raw = "Here is the plan:\n```json\n{\"goal\":\"test\",\"tasks\":[{\"id\":\"t1\",\"description\":\"x\",\"cli_type\":\"glm\",\"depends_on\":[],\"prompt\":\"y\"}]}\n```\nDone!";
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.tasks.len(), 1);
    }

    #[test]
    fn test_embedded_json() {
        let raw = "Some text {\"goal\":\"test\",\"tasks\":[]} more text";
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.goal, "test");
    }
}
