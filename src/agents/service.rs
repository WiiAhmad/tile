use crate::agents::history::build_aisdk_history;
use crate::agents::prompts::{compose_system_prompt, PromptMode};
use crate::agents::provider::{build_language_model, SelectedLanguageModel};
use crate::agents::structured::TelegramAssistantOutput;
use crate::agents::tools::l0_tools;
use crate::config::Config;
use crate::error::Result;
use crate::l0::repository::L0Repository;
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::LoggingBus;
use crate::types::TelegramMeta;
use aisdk::core::{utils::step_count_is, GenerateTextResponse, LanguageModelRequest, Messages};
use async_trait::async_trait;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

pub async fn call_ai_with_timeout_retry<F, Fut, T>(
    mut make_call: F,
    timeout: Duration,
    max_attempts: usize,
    logs: Arc<LoggingBus>,
) -> Result<T>
where
    F: FnMut(usize) -> Fut,
    Fut: Future<Output = Result<T>>,
{
    for attempt in 1..=max_attempts {
        match tokio::time::timeout(timeout, make_call(attempt)).await {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(error)) => return Err(error),
            Err(_) if attempt < max_attempts => {
                logs.emit(BotLogEvent::new(
                    LogLevel::Warn,
                    "ai.request.timeout",
                    format!("AI request attempt {attempt} timed out"),
                ))
                .await;
                tokio::time::sleep(backoff_for_attempt(attempt)).await;
            }
            Err(_) => {
                logs.emit(BotLogEvent::new(
                    LogLevel::Error,
                    "ai.request.timeout_exhausted",
                    format!("AI request timed out after {max_attempts} attempts"),
                ))
                .await;
                anyhow::bail!("AI request timed out after {max_attempts} attempts");
            }
        }
    }

    unreachable!("loop always returns or errors")
}

fn backoff_for_attempt(attempt: usize) -> Duration {
    match attempt {
        1 => Duration::from_millis(500),
        2 => Duration::from_millis(1_000),
        _ => Duration::from_millis(2_000),
    }
}

#[derive(Debug, Clone)]
pub struct AiReply {
    pub text: String,
    pub provider: String,
    pub model: String,
    pub usage: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct AiGenerateRequest {
    pub config: Config,
    pub messages: Messages,
    pub meta: TelegramMeta,
    pub l0: Arc<dyn L0Repository>,
    pub logs: Arc<LoggingBus>,
}

#[async_trait]
pub trait AiGenerator: Send + Sync {
    async fn generate(&self, request: AiGenerateRequest) -> Result<AiReply>;
}

pub struct AisdkGenerator;

#[async_trait]
impl AiGenerator for AisdkGenerator {
    async fn generate(&self, request: AiGenerateRequest) -> Result<AiReply> {
        let api_key = api_key_from_env()?;
        let model = build_language_model(&request.config, api_key)?;
        generate_with_selected_model(model, request).await
    }
}

pub struct AiService {
    pub config: Config,
    pub l0: Arc<dyn L0Repository>,
    pub logs: Arc<LoggingBus>,
    generator: Arc<dyn AiGenerator>,
}

impl AiService {
    pub fn new(config: Config, l0: Arc<dyn L0Repository>, logs: Arc<LoggingBus>) -> Self {
        Self::with_generator(config, l0, logs, Arc::new(AisdkGenerator))
    }

    pub fn with_generator(
        config: Config,
        l0: Arc<dyn L0Repository>,
        logs: Arc<LoggingBus>,
        generator: Arc<dyn AiGenerator>,
    ) -> Self {
        Self { config, l0, logs, generator }
    }

    pub async fn reply(&self, meta: &TelegramMeta) -> Result<AiReply> {
        let records = self
            .l0
            .list(&meta.conversation_id, self.config.l0_history_limit)
            .await?;
        let messages = build_aisdk_history(
            records,
            self.config.l0_max_user_history,
            self.config.l0_max_assistant_history,
        );
        let request = AiGenerateRequest {
            config: self.config.clone(),
            messages,
            meta: meta.clone(),
            l0: self.l0.clone(),
            logs: self.logs.clone(),
        };
        let generator = self.generator.clone();
        let logs = self.logs.clone();
        call_ai_with_timeout_retry(
            move |_attempt| {
                let generator = generator.clone();
                let request = request.clone();
                async move { generator.generate(request).await }
            },
            self.config.ai_agent_timeout,
            self.config.ai_agent_max_timeout_retries,
            logs,
        )
        .await
    }

    #[cfg(test)]
    pub async fn fallback_reply(&self, text: &str) -> AiReply {
        AiReply {
            text: text.to_string(),
            provider: self.config.ai_provider.as_str().to_string(),
            model: self.config.ai_model.clone(),
            usage: None,
        }
    }
}

fn api_key_from_env() -> Result<String> {
    let key = "AI_API_KEY";
    let value = std::env::var(key)?;
    if value.trim().is_empty() {
        anyhow::bail!("{key} is empty");
    }
    Ok(value)
}

async fn generate_with_selected_model(
    model: SelectedLanguageModel,
    request: AiGenerateRequest,
) -> Result<AiReply> {
    let system = compose_system_prompt(PromptMode {
        structured_output: true,
        developer_observability: false,
    });
    let provider = request.config.ai_provider.as_str().to_string();
    let model_name = request.config.ai_model.clone();
    let max_agent_steps = request.config.max_tool_failure_retries + 3;
    let text = match model {
        SelectedLanguageModel::OpenAi(model) => {
            let mut builder = LanguageModelRequest::builder()
                .model(model)
                .system(system)
                .messages(request.messages)
                .schema::<TelegramAssistantOutput>()
                .stop_when(step_count_is(max_agent_steps));
            for tool in l0_tools(request.meta, request.l0, request.logs) {
                builder = builder.with_tool(tool);
            }
            let mut request = builder.build();
            response_text(request.generate_text().await?)?
        }
        SelectedLanguageModel::OpenAiCompatible(model) => {
            let mut builder = LanguageModelRequest::builder()
                .model(model)
                .system(system)
                .messages(request.messages)
                .schema::<TelegramAssistantOutput>()
                .stop_when(step_count_is(max_agent_steps));
            for tool in l0_tools(request.meta, request.l0, request.logs) {
                builder = builder.with_tool(tool);
            }
            let mut request = builder.build();
            response_text(request.generate_text().await?)?
        }
        SelectedLanguageModel::Anthropic(model) => {
            let mut builder = LanguageModelRequest::builder()
                .model(model)
                .system(system)
                .messages(request.messages)
                .schema::<TelegramAssistantOutput>()
                .stop_when(step_count_is(max_agent_steps));
            for tool in l0_tools(request.meta, request.l0, request.logs) {
                builder = builder.with_tool(tool);
            }
            let mut request = builder.build();
            response_text(request.generate_text().await?)?
        }
    };
    Ok(AiReply {
        text,
        provider,
        model: model_name,
        usage: None,
    })
}

fn response_text(response: GenerateTextResponse) -> Result<String> {
    match response.into_schema::<TelegramAssistantOutput>() {
        Ok(output) => Ok(output.reply),
        Err(_) => response
            .text()
            .ok_or_else(|| anyhow::anyhow!("AI provider returned no text")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AiProvider;
    use crate::l0::memory_repository::MemoryL0Repository;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn config() -> Config {
        Config {
            telegram_token_present: false,
            bot_token: None,
            ai_provider: AiProvider::Anthropic,
            ai_model: "claude-sonnet-4-6".into(),
            ai_base_url: None,
            ai_api_path: None,
            l0_history_limit: 30,
            l0_max_user_history: 15,
            l0_max_assistant_history: 15,
            l0_search_limit: 10,
            max_tool_failure_retries: 5,
            ai_agent_timeout: Duration::from_secs(60),
            ai_agent_max_timeout_retries: 3,
            log_level: "info".into(),
        }
    }

    #[tokio::test]
    async fn returns_success_without_retry() {
        let logs = Arc::new(LoggingBus::default());
        let value = call_ai_with_timeout_retry(
            |_attempt| async { Ok::<_, anyhow::Error>(42) },
            Duration::from_secs(1),
            3,
            logs,
        )
        .await
        .unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn retries_timeout_until_success() {
        let logs = Arc::new(LoggingBus::default());
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let value = call_ai_with_timeout_retry(
            move |_attempt| {
                let attempts = attempts_clone.clone();
                async move {
                    let count = attempts.fetch_add(1, Ordering::SeqCst);
                    if count == 0 {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                    Ok::<_, anyhow::Error>(7)
                }
            },
            Duration::from_millis(5),
            3,
            logs,
        )
        .await
        .unwrap();
        assert_eq!(value, 7);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn errors_after_max_timeouts() {
        let logs = Arc::new(LoggingBus::default());
        let error = call_ai_with_timeout_retry(
            |_attempt| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok::<_, anyhow::Error>(())
            },
            Duration::from_millis(5),
            2,
            logs,
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(error.contains("timed out after 2 attempts"));
    }

    #[tokio::test]
    async fn fallback_reply_uses_configured_provider_and_model() {
        let service = AiService::new(
            config(),
            Arc::new(MemoryL0Repository::new()),
            Arc::new(LoggingBus::default()),
        );
        let reply = service.fallback_reply("hello").await;
        assert_eq!(reply.text, "hello");
        assert_eq!(reply.provider, "anthropic");
        assert_eq!(reply.model, "claude-sonnet-4-6");
        assert!(reply.usage.is_none());
    }

    struct FixedGenerator;

    #[async_trait::async_trait]
    impl AiGenerator for FixedGenerator {
        async fn generate(&self, request: AiGenerateRequest) -> Result<AiReply> {
            assert_eq!(request.messages.len(), 1);
            Ok(AiReply {
                text: "llm reply".to_string(),
                provider: request.config.ai_provider.as_str().to_string(),
                model: request.config.ai_model.clone(),
                usage: None,
            })
        }
    }

    #[tokio::test]
    async fn reply_loads_bounded_l0_history_and_uses_generator() {
        let repo = Arc::new(MemoryL0Repository::new());
        repo.add(crate::l0::model::L0Record::new_user(
            "user-1".to_string(),
            "telegram:1".to_string(),
            1,
            Some(2),
            Some(3),
            "hello".to_string(),
            1,
        ))
        .await
        .unwrap();
        let service = AiService::with_generator(
            config(),
            repo,
            Arc::new(LoggingBus::default()),
            Arc::new(FixedGenerator),
        );
        let meta = crate::types::TelegramMeta::from_chat(1, Some(2), Some(3));

        let reply = service.reply(&meta).await.unwrap();

        assert_eq!(reply.text, "llm reply");
        assert_eq!(reply.provider, "anthropic");
    }
}
