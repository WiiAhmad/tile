pub struct PromptSet {
    pub assistant_system: &'static str,
    pub structured_output: &'static str,
    pub structured_output_retry: &'static str,
    pub l0_memory: &'static str,
    pub tool_policy: &'static str,
    pub developer_observability: &'static str,
}

pub static PROMPTS: PromptSet = PromptSet {
    assistant_system: include_str!("prompts/assistant_system.txt"),
    structured_output: include_str!("prompts/structured_output.txt"),
    structured_output_retry: include_str!("prompts/structured_output_retry.txt"),
    l0_memory: include_str!("prompts/l0_memory.txt"),
    tool_policy: include_str!("prompts/tool_policy.txt"),
    developer_observability: include_str!("prompts/developer_observability.txt"),
};

#[derive(Debug, Clone, Copy, Default)]
pub struct PromptMode {
    pub structured_output: bool,
    pub developer_observability: bool,
}

pub fn compose_system_prompt(mode: PromptMode) -> String {
    let mut parts = vec![PROMPTS.assistant_system, PROMPTS.l0_memory, PROMPTS.tool_policy];
    if mode.structured_output {
        parts.push(PROMPTS.structured_output);
        parts.push(PROMPTS.structured_output_retry);
    }
    if mode.developer_observability {
        parts.push(PROMPTS.developer_observability);
    }
    parts.join("\n\n---\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_prompt_includes_memory_and_tools() {
        let prompt = compose_system_prompt(PromptMode::default());
        assert!(prompt.contains("Telegram AI assistant"));
        assert!(prompt.contains("L0 memory"));
        assert!(prompt.contains("L0 tools"));
    }

    #[test]
    fn structured_prompt_includes_json_rules() {
        let prompt = compose_system_prompt(PromptMode { structured_output: true, developer_observability: false });
        assert!(prompt.contains("only valid JSON"));
    }
}
