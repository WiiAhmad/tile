use crate::config::{AiProvider, Config};
use crate::error::Result;
use aisdk::core::DynamicModel;
use aisdk::providers::{Anthropic, OpenAI, OpenAICompatible};

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedProvider {
    pub provider: AiProvider,
    pub model: String,
    pub base_url: Option<String>,
    pub api_path: Option<String>,
}

#[cfg(test)]
pub fn selected_provider(config: &Config) -> SelectedProvider {
    SelectedProvider {
        provider: config.ai_provider.clone(),
        model: config.ai_model.clone(),
        base_url: config.ai_base_url.clone(),
        api_path: config.ai_api_path.clone(),
    }
}

#[derive(Debug, Clone)]
pub enum SelectedLanguageModel {
    OpenAi(OpenAI<DynamicModel>),
    OpenAiCompatible(OpenAICompatible<DynamicModel>),
    Anthropic(Anthropic<DynamicModel>),
}

pub fn build_language_model(
    config: &Config,
    api_key: impl Into<String>,
) -> Result<SelectedLanguageModel> {
    let api_key = api_key.into();
    match config.ai_provider {
        AiProvider::OpenAi => {
            let mut builder = OpenAI::<DynamicModel>::builder()
                .model_name(config.ai_model.clone())
                .api_key(api_key);
            if let Some(base_url) = &config.ai_base_url {
                builder = builder.base_url(base_url.clone());
            }
            if let Some(api_path) = &config.ai_api_path {
                builder = builder.path(api_path.clone());
            }
            Ok(SelectedLanguageModel::OpenAi(builder.build()?))
        }
        AiProvider::OpenAiCompatible => {
            let mut builder = OpenAICompatible::<DynamicModel>::builder()
                .model_name(config.ai_model.clone())
                .api_key(api_key);
            if let Some(base_url) = &config.ai_base_url {
                builder = builder.base_url(base_url.clone());
            }
            if let Some(api_path) = &config.ai_api_path {
                builder = builder.path(api_path.clone());
            }
            Ok(SelectedLanguageModel::OpenAiCompatible(builder.build()?))
        }
        AiProvider::Anthropic => {
            let mut builder = Anthropic::<DynamicModel>::builder()
                .model_name(config.ai_model.clone())
                .api_key(api_key);
            if let Some(base_url) = &config.ai_base_url {
                builder = builder.base_url(base_url.clone());
            }
            if let Some(api_path) = &config.ai_api_path {
                builder = builder.path(api_path.clone());
            }
            Ok(SelectedLanguageModel::Anthropic(builder.build()?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn config() -> Config {
        Config {
            telegram_token_present: false,
            iii_url: "ws://127.0.0.1:49134".into(),
            ai_provider: AiProvider::Anthropic,
            ai_model: "claude-sonnet-4-6".into(),
            ai_base_url: Some("https://proxy.example.com/v1".into()),
            ai_api_path: Some("/messages".into()),
            l0_history_limit: 30,
            l0_max_user_history: 15,
            l0_max_assistant_history: 15,
            l0_search_limit: 10,
            l0_use_worker_functions: false,
            l0_fts_sqlite_path: "./data/iii.db".to_string(),
            health_check_interval: Duration::from_secs(60),
            db_health_timeout: Duration::from_millis(2000),
            tool_audit_log_to_l0: true,
            max_tool_failure_retries: 5,
            ai_agent_timeout: Duration::from_secs(60),
            ai_agent_max_timeout_retries: 3,
            log_level: "info".into(),
            log_to_terminal: true,
            log_to_jsonl: true,
            log_jsonl_path: "./logs/bot-events.jsonl".into(),
            log_to_database: true,
            log_to_pubsub: true,
            log_pubsub_topic: "bot.logs".into(),
        }
    }

    #[test]
    fn returns_provider_model_and_endpoint_from_config() {
        let selected = selected_provider(&config());
        assert_eq!(selected.provider, AiProvider::Anthropic);
        assert_eq!(selected.model, "claude-sonnet-4-6");
        assert_eq!(selected.base_url.as_deref(), Some("https://proxy.example.com/v1"));
        assert_eq!(selected.api_path.as_deref(), Some("/messages"));
    }

    #[test]
    fn custom_openai_base_url_without_api_path_keeps_provider_default_path() {
        let mut config = config();
        config.ai_provider = AiProvider::OpenAi;
        config.ai_base_url = Some("https://proxy.example.com/v1".into());
        config.ai_api_path = None;

        let model = build_language_model(&config, "test-key").unwrap();

        match model {
            SelectedLanguageModel::OpenAi(model) => {
                assert_eq!(model.settings.base_url, "https://proxy.example.com/v1");
                assert_eq!(model.settings.path.as_deref(), None);
            }
            other => panic!("expected OpenAI model, got {other:?}"),
        }
    }

    #[test]
    fn openai_compatible_base_url_without_api_path_uses_chat_completions_default() {
        let mut config = config();
        config.ai_provider = AiProvider::OpenAiCompatible;
        config.ai_model = "custom-model".into();
        config.ai_base_url = Some("https://proxy.example.com/v1".into());
        config.ai_api_path = None;

        let model = build_language_model(&config, "test-key").unwrap();

        match model {
            SelectedLanguageModel::OpenAiCompatible(model) => {
                assert_eq!(model.settings.base_url, "https://proxy.example.com/v1");
                assert_eq!(model.settings.path.as_deref(), None);
            }
            other => panic!("expected OpenAI-compatible model, got {other:?}"),
        }
    }

    #[test]
    fn builds_openai_compatible_model_with_custom_endpoint() {
        let mut config = config();
        config.ai_provider = AiProvider::OpenAiCompatible;
        config.ai_model = "custom-model".into();
        config.ai_base_url = Some("https://proxy.example.com/v1".into());
        config.ai_api_path = Some("/chat/completions".into());

        let model = build_language_model(&config, "test-key").unwrap();

        match model {
            SelectedLanguageModel::OpenAiCompatible(model) => {
                assert_eq!(model.settings.base_url, "https://proxy.example.com/v1");
                assert_eq!(model.settings.api_key, "test-key");
                assert_eq!(model.settings.path.as_deref(), Some("/chat/completions"));
            }
            other => panic!("expected OpenAI-compatible model, got {other:?}"),
        }
    }

    #[test]
    fn builds_anthropic_model_with_custom_endpoint() {
        let model = build_language_model(&config(), "test-key").unwrap();

        match model {
            SelectedLanguageModel::Anthropic(model) => {
                assert_eq!(model.settings.base_url, "https://proxy.example.com/v1");
                assert_eq!(model.settings.api_key, "test-key");
                assert_eq!(model.settings.path.as_deref(), Some("/messages"));
            }
            other => panic!("expected Anthropic model, got {other:?}"),
        }
    }
}
