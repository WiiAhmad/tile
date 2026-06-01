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

    pub fn base_url_env_key(&self) -> &'static str {
        match self {
            Self::OpenAi | Self::OpenAiCompatible => "OPENAI_BASE_URL",
            Self::Anthropic => "ANTHROPIC_BASE_URL",
        }
    }

    pub fn api_path_env_key(&self) -> &'static str {
        match self {
            Self::OpenAi | Self::OpenAiCompatible => "OPENAI_API_PATH",
            Self::Anthropic => "ANTHROPIC_API_PATH",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub telegram_token_present: bool,
    pub iii_url: String,
    pub ai_provider: AiProvider,
    pub ai_model: String,
    pub ai_base_url: Option<String>,
    pub ai_api_path: Option<String>,
    pub l0_history_limit: usize,
    pub l0_max_user_history: usize,
    pub l0_max_assistant_history: usize,
    pub l0_search_limit: usize,
    pub health_check_interval: Duration,
    pub db_health_timeout: Duration,
    pub tool_audit_log_to_l0: bool,
    pub max_tool_failure_retries: usize,
    pub ai_agent_timeout: Duration,
    pub ai_agent_max_timeout_retries: usize,
    pub log_level: String,
    pub log_to_terminal: bool,
    pub log_to_jsonl: bool,
    pub log_jsonl_path: String,
    pub log_to_database: bool,
    pub log_to_pubsub: bool,
    pub log_pubsub_topic: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let ai_provider = AiProvider::parse(&env_string("AI_PROVIDER", "anthropic"))?;
        let default_model = match ai_provider {
            AiProvider::Anthropic => "claude-sonnet-4-6",
            AiProvider::OpenAi => "gpt-5",
            AiProvider::OpenAiCompatible => "gpt-5-nano",
        };
        let ai_base_url = env_optional(ai_provider.base_url_env_key());
        let ai_api_path = env_optional(ai_provider.api_path_env_key());

        Ok(Self {
            telegram_token_present: env::var("TELOXIDE_TOKEN").is_ok(),
            iii_url: env_string("III_URL", "ws://127.0.0.1:49134"),
            ai_provider,
            ai_model: env_string("AI_MODEL", default_model),
            ai_base_url,
            ai_api_path,
            l0_history_limit: env_usize("L0_HISTORY_LIMIT", 30)?,
            l0_max_user_history: env_usize("L0_MAX_USER_HISTORY", 15)?,
            l0_max_assistant_history: env_usize("L0_MAX_ASSISTANT_HISTORY", 15)?,
            l0_search_limit: env_usize("L0_SEARCH_LIMIT", 10)?,
            health_check_interval: Duration::from_secs(env_u64("HEALTH_CHECK_INTERVAL_SECS", 60)?),
            db_health_timeout: Duration::from_millis(env_u64("DB_HEALTH_TIMEOUT_MS", 2_000)?),
            tool_audit_log_to_l0: env_bool("TOOL_AUDIT_LOG_TO_L0", true)?,
            max_tool_failure_retries: env_usize("MAX_TOOL_FAILURE_RETRIES", 5)?,
            ai_agent_timeout: Duration::from_secs(env_u64("AI_AGENT_TIMEOUT_SECS", 60)?),
            ai_agent_max_timeout_retries: env_usize("AI_AGENT_MAX_TIMEOUT_RETRIES", 3)?,
            log_level: env_string("LOG_LEVEL", "info"),
            log_to_terminal: env_bool("LOG_TO_TERMINAL", true)?,
            log_to_jsonl: env_bool("LOG_TO_JSONL", true)?,
            log_jsonl_path: env_string("LOG_JSONL_PATH", "./logs/bot-events.jsonl"),
            log_to_database: env_bool("LOG_TO_DATABASE", true)?,
            log_to_pubsub: env_bool("LOG_TO_PUBSUB", true)?,
            log_pubsub_topic: env_string("LOG_PUBSUB_TOPIC", "bot.logs"),
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

fn env_bool(key: &str, default: bool) -> Result<bool> {
    match env::var(key) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            other => anyhow::bail!("invalid bool for {key}: {other}"),
        },
        Err(_) => Ok(default),
    }
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
    fn exposes_provider_specific_endpoint_env_keys() {
        assert_eq!(AiProvider::OpenAi.base_url_env_key(), "OPENAI_BASE_URL");
        assert_eq!(AiProvider::OpenAi.api_path_env_key(), "OPENAI_API_PATH");
        assert_eq!(
            AiProvider::OpenAiCompatible.base_url_env_key(),
            "OPENAI_BASE_URL"
        );
        assert_eq!(
            AiProvider::OpenAiCompatible.api_path_env_key(),
            "OPENAI_API_PATH"
        );
        assert_eq!(
            AiProvider::Anthropic.base_url_env_key(),
            "ANTHROPIC_BASE_URL"
        );
        assert_eq!(
            AiProvider::Anthropic.api_path_env_key(),
            "ANTHROPIC_API_PATH"
        );
    }

    #[test]
    fn rejects_unknown_provider() {
        let error = AiProvider::parse("local-llm").unwrap_err().to_string();
        assert!(error.contains("unsupported AI_PROVIDER"));
    }
}
