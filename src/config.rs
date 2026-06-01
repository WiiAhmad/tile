use crate::error::Result;
use std::env;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiProvider {
    OpenAi,
    OpenAiCompatible,
    Anthropic,
}

impl AiProvider {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" | "open_ai" => Ok(Self::OpenAi),
            "openai_compatible" | "openai-compatible" | "open_ai_compatible" | "compatible" => {
                Ok(Self::OpenAiCompatible)
            }
            "anthropic" | "claude" => Ok(Self::Anthropic),
            other => anyhow::bail!("unsupported AI_PROVIDER: {other}"),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::OpenAiCompatible => "openai_compatible",
            Self::Anthropic => "anthropic",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: Option<String>,
    pub ai_provider: AiProvider,
    pub ai_model: String,
    pub ai_base_url: Option<String>,
    pub ai_api_path: Option<String>,
    pub l0_history_limit: usize,
    pub l0_max_user_history: usize,
    pub l0_max_assistant_history: usize,
    pub l0_search_limit: usize,
    pub max_tool_failure_retries: usize,
    pub ai_agent_timeout: Duration,
    pub ai_agent_max_timeout_retries: usize,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let ai_provider = AiProvider::parse(&env_string("AI_PROVIDER", "anthropic"))?;
        let default_model = match ai_provider {
            AiProvider::Anthropic => "claude-sonnet-4-6",
            AiProvider::OpenAi => "gpt-5",
            AiProvider::OpenAiCompatible => "gpt-5-nano",
        };
        let ai_base_url = env_optional("AI_BASE_URL");
        let ai_api_path = env_optional("AI_API_PATH");

        let bot_token = env_optional("BOT_TOKEN");

        Ok(Self {
            bot_token,
            ai_provider,
            ai_model: env_string("AI_MODEL", default_model),
            ai_base_url,
            ai_api_path,
            l0_history_limit: env_usize("L0_HISTORY_LIMIT", 30)?,
            l0_max_user_history: env_usize("L0_MAX_USER_HISTORY", 15)?,
            l0_max_assistant_history: env_usize("L0_MAX_ASSISTANT_HISTORY", 15)?,
            l0_search_limit: env_usize("L0_SEARCH_LIMIT", 10)?,
            max_tool_failure_retries: env_usize("MAX_TOOL_FAILURE_RETRIES", 5)?,
            ai_agent_timeout: Duration::from_secs(env_u64("AI_AGENT_TIMEOUT_SECS", 60)?),
            ai_agent_max_timeout_retries: env_usize("AI_AGENT_MAX_TIMEOUT_RETRIES", 3)?,
            log_level: env_string("LOG_LEVEL", "info"),
        })
    }
}

fn env_string(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_u64(key: &str, default: u64) -> Result<u64> {
    match env::var(key) {
        Ok(value) => Ok(value.parse::<u64>()?),
        Err(_) => Ok(default),
    }
}

fn env_usize(key: &str, default: usize) -> Result<usize> {
    match env::var(key) {
        Ok(value) => Ok(value.parse::<usize>()?),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ai_provider_aliases() {
        assert_eq!(AiProvider::parse("anthropic").unwrap(), AiProvider::Anthropic);
        assert_eq!(AiProvider::parse("claude").unwrap(), AiProvider::Anthropic);
        assert_eq!(AiProvider::parse("openai").unwrap(), AiProvider::OpenAi);
        assert_eq!(AiProvider::parse("open_ai").unwrap(), AiProvider::OpenAi);
        assert_eq!(
            AiProvider::parse("openai_compatible").unwrap(),
            AiProvider::OpenAiCompatible
        );
        assert_eq!(
            AiProvider::parse("openai-compatible").unwrap(),
            AiProvider::OpenAiCompatible
        );
    }

    #[test]
    fn bot_token_env_is_loaded() {
        unsafe {
            env::remove_var("BOT_TOKEN");
            env::set_var("BOT_TOKEN", "123:test");
        }
        let config = Config::from_env().unwrap();
        unsafe {
            env::remove_var("BOT_TOKEN");
        }

        assert_eq!(config.bot_token.as_deref(), Some("123:test"));
    }

    #[test]
    fn ignores_tool_audit_env() {
        unsafe {
            env::set_var("REMOVED_BOOLEAN_CONFIG", "not-a-bool");
        }
        let result = Config::from_env();
        unsafe {
            env::remove_var("REMOVED_BOOLEAN_CONFIG");
        }

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_unknown_provider() {
        let error = AiProvider::parse("local-llm").unwrap_err().to_string();
        assert!(error.contains("unsupported AI_PROVIDER"));
    }
}
