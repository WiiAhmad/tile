# Telegram AI L0 Bot — Specs and Implementation Plan

## Summary

Build a Rust Telegram bot using `teloxide` that connects Telegram conversations to an AI agent powered by the Rust `aisdk` crate. The bot stores raw conversation history as **L0 memory** through `iii-sdk`, backed by SQLite through the iii runtime configuration. The AI receives recent conversation history as `aisdk` messages and can use custom tools to add, search, and list L0 records. Structured output must work in a provider-neutral way with both OpenAI and Anthropic models.

Current project dependencies already include:

- `teloxide = "0.17.0"` with `macros`
- `aisdk = "0.5.2"` with `openai` and `anthropic`
- `iii-sdk = "0.16.1"`
- `serde`, `schemars`, `tokio`, `dotenv`, and `log`

Current iii config already includes a SQLite database entry:

```yaml
- name: database
  config:
    databases:
      primary:
        url: sqlite:./iii.db
```

## Goals

1. Receive Telegram messages with `teloxide`.
2. Store user and assistant events as raw L0 records.
3. Load recent L0 records and adapt them into `aisdk` chat history.
4. Call OpenAI or Anthropic through `aisdk`.
5. Support custom AI tools:
   - `l0_add`
   - `l0_search`
   - `l0_list`
6. Persist L0 records through `iii-sdk` and iii runtime storage.
7. Monitor database and L0 backend health at startup and during runtime.
8. Support structured output through `aisdk` schemas using `schemars`.
9. Add tool lifecycle hooks:
   - pre-tool-use hook for context injection and policy checks
   - post-tool-use hook for success logging and L0 audit records
   - post-tool-failure hook for failure logging and L0 audit records
10. Add multi-sink logging:
   - human-readable terminal logs
   - JSONL file persistence
   - database/L0 persistence
   - iii pubsub publishing
   - local WebSocket live stream
11. Keep the design provider-neutral and Telegram-adapter-neutral where practical.

## Non-goals for the first version

- No global cross-chat memory search.
- No automatic long-term summarization layer above L0.
- No vector search requirement yet.
- No admin panel.
- No multi-platform adapter beyond Telegram.
- No rich Telegram UI beyond basic commands and text replies.

## Architecture

```text
Telegram User
    |
    v
teloxide Bot
    |
    v
Telegram Message Handler
    |
    +--> L0 repository: store raw user Telegram message
    |
    +--> L0 repository: load recent/searchable conversation history
    |
    +--> aisdk Message::conversation_builder()
    |
    +--> aisdk LanguageModelRequest
            |
            +--> OpenAI or Anthropic provider
            |
            +--> Custom L0 tools
                    |
                    +--> pre_tool_use: inject trusted context and enforce scope
                    +--> tool implementation: l0_add / l0_search / l0_list
                    +--> post_tool_use: log success and audit to L0
                    +--> post_tool_failure: log failure and audit to L0
    |
    +--> Health monitor: database, iii, and L0 backend status
    |
    +--> LoggingBus
    |       +--> terminal
    |       +--> JSONL file
    |       +--> database/L0
    |       +--> iii pubsub
    |       +--> WebSocket stream
    |
    +--> L0 repository: store assistant response and tool events
    |
    v
Telegram send_message()
```

## Proposed module layout

Use folders by component so files stay small and readable. Avoid putting all Telegram, AI, logging, and L0 code into one large file.

```text
src/
  main.rs                         # application entrypoint and wiring only
  config.rs                       # env/config loading
  error.rs                        # shared Result/Error aliases if needed
  types.rs                        # shared domain types and structured output schemas

  agents/
    mod.rs                        # exports agent components
    service.rs                    # AiService orchestration
    provider.rs                   # OpenAI/Anthropic provider/model selection
    history.rs                    # L0 -> aisdk message window adaptation
    structured.rs                 # structured output schemas/retry behavior
    prompts.rs                    # prompt loading/composition helpers
    prompts/
      assistant_system.txt        # main Telegram assistant prompt
      structured_output.txt       # schema-only response rules
      structured_output_retry.txt # schema repair prompt
      l0_memory.txt               # memory behavior rules
      tool_policy.txt             # tool scope/retry behavior
      developer_observability.txt # debug/observability behavior
    tools.rs                      # aisdk tool registration
    tool_hooks.rs                 # pre/post/failure tool lifecycle hooks
    tool_loop.rs                  # tool-call retry/step budget enforcement

  telegram/
    mod.rs                        # exports Telegram components
    dispatcher.rs                 # teloxide dispatcher setup
    handlers.rs                   # message and command handlers
    commands.rs                   # BotCommands enum and parsing
    format.rs                     # Telegram output formatting helpers

  l0/
    mod.rs                        # exports L0 components
    repository.rs                 # L0Repository trait
    iii_repository.rs             # iii-sdk implementation
    model.rs                      # L0Record, roles, sources
    search.rs                     # naive search and future FTS hooks

  health/
    mod.rs                        # exports health monitor components
    monitor.rs                    # periodic checks and latest report cache
    checks.rs                     # iii/database/L0 checks
    model.rs                      # HealthReport, HealthCheck, HealthStatus

  logging/
    mod.rs                        # exports logging components
    events.rs                     # structured log event schema
    terminal.rs                   # human-readable terminal logging
    jsonl.rs                      # append-only JSONL file sink
    pubsub.rs                     # iii-pubsub publishing sink
    websocket.rs                  # websocket log stream for external tools
    redaction.rs                  # secrets/content redaction helpers
```

## Configuration

Use environment variables for runtime configuration.

```env
TELOXIDE_TOKEN=...

III_URL=ws://127.0.0.1:49134

AI_PROVIDER=anthropic
AI_MODEL=claude-sonnet-4-6

ANTHROPIC_API_KEY=...
OPENAI_API_KEY=...

L0_HISTORY_LIMIT=30
L0_MAX_USER_HISTORY=15
L0_MAX_ASSISTANT_HISTORY=15
L0_SEARCH_LIMIT=10

HEALTH_CHECK_INTERVAL_SECS=60
DB_HEALTH_TIMEOUT_MS=2000
TOOL_AUDIT_LOG_TO_L0=true
MAX_TOOL_FAILURE_RETRIES=5
AI_AGENT_TIMEOUT_SECS=60
AI_AGENT_MAX_TIMEOUT_RETRIES=3

LOG_LEVEL=info
LOG_TO_TERMINAL=true
LOG_TO_JSONL=true
LOG_JSONL_PATH=./logs/bot-events.jsonl
LOG_TO_DATABASE=true
LOG_TO_PUBSUB=true
LOG_PUBSUB_TOPIC=bot.logs
LOG_WEBSOCKET_ENABLED=true
LOG_WEBSOCKET_HOST=127.0.0.1
LOG_WEBSOCKET_PORT=3120
```

Supported providers:

```rust
pub enum AiProvider {
    OpenAi,
    Anthropic,
}
```

Initial provider defaults:

| Provider | Default model helper |
| --- | --- |
| Anthropic | `Anthropic::claude_4_sonnet()` |
| OpenAI | `OpenAI::gpt_5()` or `OpenAI::gpt_4o()` |

The exact helper should be selected based on what compiles cleanly with the installed `aisdk` version.

## L0 memory definition

L0 is the rawest durable memory layer. It stores original conversation and event data with minimal transformation.

L0 records should include:

- Telegram user messages
- Assistant responses
- Tool call requests
- Tool call results
- System/meta events if useful
- Raw Telegram identifiers
- Provider/model metadata

L0 should not summarize, deduplicate, rewrite, or compress messages. Higher-level memory layers can be added later.

## L0 record schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct L0Record {
    pub id: String,
    pub conversation_id: String,

    pub telegram_chat_id: i64,
    pub telegram_user_id: Option<u64>,
    pub telegram_message_id: Option<i32>,

    pub role: L0Role,
    pub content: String,
    pub source: L0Source,

    pub provider: Option<String>,
    pub model: Option<String>,

    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,

    pub raw_json: Option<serde_json::Value>,
    pub created_at_ms: i64,
}
```

Roles:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum L0Role {
    System,
    User,
    Assistant,
    Tool,
    Telegram,
}
```

Sources:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum L0Source {
    TelegramUpdate,
    AiRequest,
    AiResponse,
    ToolCall,
    ToolResult,
    ToolFailure,
    LogEvent,
    HealthCheck,
    Manual,
}
```

Conversation IDs should be scoped by Telegram chat:

```text
telegram:<chat_id>
```

Examples:

```text
telegram:123456789
telegram:-1001234567890
```

## L0 repository abstraction

Define a trait so iii-backed storage can be replaced or supplemented later.

```rust
#[async_trait]
pub trait L0Repository {
    async fn add(&self, record: L0Record) -> Result<()>;

    async fn list(
        &self,
        conversation_id: &str,
        limit: usize,
    ) -> Result<Vec<L0Record>>;

    async fn search(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<L0Record>>;
}
```

First implementation:

```rust
pub struct IiiL0Repository {
    iii: iii_sdk::ClientLike,
}
```

The exact `iii-sdk` client type should be finalized during implementation based on the crate API.

## iii storage strategy

Use iii stream operations as the first durable interface.

```text
stream_name = "telegram_l0"
group_id    = conversation_id
item_id     = record.id
data        = serialized L0Record
```

Example payload:

```json
{
  "stream_name": "telegram_l0",
  "group_id": "telegram:123456789",
  "item_id": "018f...",
  "data": {
    "id": "018f...",
    "conversation_id": "telegram:123456789",
    "role": "user",
    "source": "telegram_update",
    "content": "hello",
    "telegram_chat_id": 123456789,
    "created_at_ms": 1780000000000
  }
}
```

The iii docs show stream operations using `TriggerRequest` with functions like:

- `stream::set`
- `stream::get`
- `stream::list`
- `stream::delete`

If iii stream supports a SQLite/database adapter, configure it so L0 stream data lands in SQLite. If not, use the iii stream as the canonical API first and add a local SQLite L0 repository later behind the same trait.

## Health monitoring

Add a health monitor that checks the database, iii connection, and L0 repository path before and during bot execution.

### Health targets

| Target | Check | Healthy when |
| --- | --- | --- |
| iii runtime | trigger lightweight iii request or ping equivalent | request succeeds before timeout |
| SQLite/database | execute lightweight database check through iii database function or repository-specific ping | query succeeds before timeout |
| L0 repository write path | write a small health-check L0 or dedicated health stream item | write succeeds before timeout |
| L0 repository read path | read the health-check item or list a small number of records | read succeeds before timeout |
| AI provider config | validate provider/model/API-key presence only | required config exists; do not call model on every health tick |
| Telegram bot config | validate `TELOXIDE_TOKEN` exists at startup | token exists; Telegram API failures are reported by dispatcher logs |

### Health state type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall: HealthStatus,
    pub checked_at_ms: i64,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}
```

### Health monitor behavior

- Run startup health checks before dispatching Telegram updates.
- If iii/database/L0 is unhealthy at startup, fail fast unless `ALLOW_DEGRADED_START=true` is added later.
- Run periodic checks every `HEALTH_CHECK_INTERVAL_SECS` in a background Tokio task.
- Cache the latest `HealthReport` in shared state for the `/health` command.
- Log health transitions only when status changes to avoid noisy logs.
- Never include secrets or full user message content in health logs.

### `/health` command output

Example Telegram response:

```text
Health: degraded
- iii_runtime: healthy (12ms)
- sqlite_database: healthy (7ms)
- l0_write: unhealthy (timeout)
- l0_read: healthy (9ms)
- ai_provider_config: healthy
```

### Health monitor module shape

```rust
pub struct HealthMonitor {
    l0: Arc<dyn L0Repository + Send + Sync>,
    config: Config,
    latest: Arc<RwLock<HealthReport>>,
}

impl HealthMonitor {
    pub async fn check_once(&self) -> HealthReport;
    pub async fn run_periodic(self: Arc<Self>);
    pub async fn latest(&self) -> HealthReport;
}
```

## Search strategy

Version 1 search:

1. Call `list(conversation_id, max_scan_limit)`.
2. Perform case-insensitive substring matching in Rust.
3. Return the newest matching records up to `limit`.

Version 2 search:

- Use iii memory search if available and suitable.
- Or add SQLite FTS over L0 content.
- Or add embeddings/vector search as a higher-level L1/L2 memory feature.

## Telegram behavior

### Commands

Use `teloxide` command macros.

```rust
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "show help")]
    Help,

    #[command(description = "start the bot")]
    Start,

    #[command(description = "show current AI provider and model")]
    Model,

    #[command(description = "show database, iii, and L0 backend health")]
    Health,

    #[command(description = "list recent L0 records")]
    L0List,

    #[command(description = "search L0 memory")]
    L0Search(String),
}
```

### Normal message flow

```text
Telegram text message arrives
    |
    v
Extract chat_id, user_id, message_id, text
    |
    v
Store user message as L0
    |
    v
Load recent L0 history
    |
    v
Convert L0 to aisdk messages
    |
    v
Call AI provider
    |
    v
Store assistant response as L0
    |
    v
Reply to Telegram
```

### Group chat behavior

Initial behavior should be conservative:

- In private chats: respond to normal text.
- In groups/supergroups: respond only to commands or explicit bot mentions.

This prevents the bot from storing and responding to every group message unexpectedly.

## teloxide dispatcher plan

Use a dispatcher with command and text branches.

```rust
let handler = Update::filter_message()
    .branch(
        dptree::entry()
            .filter_command::<Command>()
            .endpoint(handle_command),
    )
    .branch(Message::filter_text().endpoint(handle_text));

Dispatcher::builder(bot, handler)
    .dependencies(dptree::deps![ai_service, l0_repository, config])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
```

## AI history adaptation

The `aisdk` docs show dynamic conversation building through `Message::conversation_builder()`. Because message history can grow very large in active Telegram chats, the bot must cap the history window before building `aisdk` messages.

### History window rule

For every AI call, include at most:

```text
15 newest user messages
15 newest assistant messages
```

Defaults:

```env
L0_MAX_USER_HISTORY=15
L0_MAX_ASSISTANT_HISTORY=15
```

Selection behavior:

1. Load recent L0 records for the conversation.
2. Keep only `User`, `Assistant`, and optional pinned `System` records for normal message history.
3. Select the newest 15 user records and newest 15 assistant records.
4. Merge those selected records back into chronological order.
5. Build `aisdk` messages from that bounded list.
6. Keep all older records in L0 so the AI can recover them through `l0_search` or `l0_list` tools.

This keeps the provider request small while preserving full raw history in the database.

L0-to-AI conversion:

```rust
pub fn build_aisdk_history(records: Vec<L0Record>, max_user: usize, max_assistant: usize) -> Messages {
    let window = select_bounded_history(records, max_user, max_assistant);
    let mut builder = Message::conversation_builder();

    for record in window {
        builder = match record.role {
            L0Role::User => builder.user(record.content),
            L0Role::Assistant => builder.assistant(record.content),
            L0Role::System => builder.system(record.content),
            _ => builder,
        };
    }

    builder.build()
}
```

Version 1 should send only `system`, `user`, and `assistant` records as normal chat history. Tool events stay in L0 and are searchable through tools.

## AI system prompts

Keep prompts in separate files so Rust source files do not become long and hard to maintain. The bot should load prompt text from prompt files at startup or embed them with `include_str!`.

Prompt files:

```text
docs/superpowers/prompts/
  assistant-system.md           # main Telegram assistant behavior
  structured-output.md          # JSON/schema-only output rules and repair prompt
  l0-memory.md                  # L0 memory usage rules
  tool-policy.md                # L0 tool scope and retry policy
  developer-observability.md    # development/debug observability guidance
```

Recommended runtime mirror for production code:

```text
src/agents/prompts/
  assistant_system.txt
  structured_output.txt
  structured_output_retry.txt
  l0_memory.txt
  tool_policy.txt
  developer_observability.txt
```

Suggested prompt loader:

```rust
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
```

Prompt composition:

```rust
pub fn compose_system_prompt(mode: PromptMode) -> String {
    let mut parts = vec![
        PROMPTS.assistant_system,
        PROMPTS.l0_memory,
        PROMPTS.tool_policy,
    ];

    if mode.structured_output {
        parts.push(PROMPTS.structured_output);
    }

    if mode.developer_observability {
        parts.push(PROMPTS.developer_observability);
    }

    parts.join("\n\n---\n\n")
}
```

Normal chat should use:

```text
assistant-system + l0-memory + tool-policy
```

Structured output should use:

```text
assistant-system + l0-memory + tool-policy + structured-output
```

Structured output retry should append:

```text
structured-output-retry
```

## AI agent timeout and retry policy

Every AI agent/provider call must have a timeout so Telegram handlers do not hang forever.

Defaults:

```env
AI_AGENT_TIMEOUT_SECS=60
AI_AGENT_MAX_TIMEOUT_RETRIES=3
```

Rules:

- Wrap each AI call with `tokio::time::timeout`.
- If the call succeeds before timeout, return the response immediately.
- If the call times out, log `ai.request.timeout` and retry.
- Retry timeout failures up to 3 attempts total.
- Use a short backoff between retries, for example 500ms, 1000ms, then 2000ms.
- If all 3 attempts time out, return a user-facing Telegram error.
- Do not duplicate the user L0 message on retries; store the user message once before the first attempt.
- Store each timeout attempt as an AI request/timeout audit event.
- Store the final timeout failure as an L0/log event.

User-facing final timeout error:

```text
The AI request timed out after several attempts. Please try again.
```

Suggested helper:

```rust
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
                // log ai.request.timeout and sleep with backoff
            }
            Err(_) => {
                // log ai.request.timeout_exhausted
                return Err(anyhow::anyhow!("AI request timed out after {max_attempts} attempts"));
            }
        }
    }

    unreachable!()
}
```

Timeout retry applies to the whole AI agent call, including any provider-side tool loop. Tool failure budget is still separate and remains capped at 5 failed tool calls per AI request.

## AI service interface

```rust
pub struct AiService {
    config: Config,
    l0: Arc<dyn L0Repository + Send + Sync>,
    logs: Arc<LoggingBus>,
}

pub struct AiReply {
    pub text: String,
    pub provider: String,
    pub model: String,
    pub usage: Option<AiUsage>,
}

impl AiService {
    pub async fn reply_to_telegram_message(
        &self,
        conversation_id: String,
        user_text: String,
        telegram_meta: TelegramMeta,
    ) -> Result<AiReply>;

    pub async fn structured_reply<T>(
        &self,
        conversation_id: String,
        user_text: String,
    ) -> Result<T>
    where
        T: DeserializeOwned + JsonSchema;
}
```

## Structured output

Use `schemars::JsonSchema` and `serde::Deserialize` with `aisdk` schemas.

Example output type:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TelegramAssistantOutput {
    pub reply: String,
    pub should_store_memory: bool,
    pub memory_tags: Vec<String>,
}
```

Expected `aisdk` usage:

```rust
let result = LanguageModelRequest::builder()
    .model(model)
    .system(system_prompt)
    .messages(messages)
    .schema::<TelegramAssistantOutput>()
    .temperature(20u32)
    .build()
    .generate_text()
    .await?;

let output: TelegramAssistantOutput = result.into_schema()?;
```

Structured output compatibility rules:

- Prefer simple JSON objects.
- Use strings, booleans, numbers, arrays, enums, and shallow nested objects.
- Avoid recursive schemas.
- Avoid provider-specific JSON schema tricks.
- Avoid `oneOf`-heavy schemas in the first version.
- Retry once on parse failure with a stricter JSON-only instruction.

## Custom AI tools

The `aisdk` docs show a `#[tool]` macro that creates tool definitions from Rust functions. Async tools can accept a `ToolContext`.

### `l0_add`

Purpose: store a raw L0 memory/event scoped to the current Telegram conversation.

Input:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct L0AddInput {
    pub content: String,
    pub role: Option<L0Role>,
    pub tags: Option<Vec<String>>,
}
```

Response:

```json
{
  "ok": true,
  "id": "record-id"
}
```

Behavior:

- Uses current conversation scope from tool context.
- Defaults role to `tool` or a dedicated memory role if no role is provided.
- Stores the tool request and result as L0 records.

### `l0_search`

Purpose: search prior raw conversation records.

Input:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct L0SearchInput {
    pub query: String,
    pub limit: Option<u32>,
}
```

Response:

```json
{
  "results": [
    {
      "id": "record-id",
      "role": "user",
      "content": "matching message text",
      "created_at_ms": 1780000000000
    }
  ]
}
```

Behavior:

- Scoped to the current conversation by default.
- Enforces max result limit.
- Does not search other Telegram chats.

### `l0_list`

Purpose: list recent raw L0 records.

Input:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct L0ListInput {
    pub limit: Option<u32>,
}
```

Response:

```json
{
  "records": [
    {
      "id": "record-id",
      "role": "assistant",
      "content": "recent assistant message",
      "created_at_ms": 1780000000000
    }
  ]
}
```

Behavior:

- Default limit: `10`.
- Max limit: `50`.
- Scoped to the current conversation.

## Logging and observability

Logging must make the bot easy to watch from the CLI terminal while also persisting structured events for later inspection and exposing live events to other tools.

### Logging sinks

| Sink | Purpose | Config |
| --- | --- | --- |
| Terminal | See live bot activity while running `cargo run` | `LOG_TO_TERMINAL=true` |
| JSONL file | Durable append-only event log for local debugging | `LOG_TO_JSONL=true`, `LOG_JSONL_PATH=./logs/bot-events.jsonl` |
| Database/L0 | Durable queryable audit trail | `LOG_TO_DATABASE=true` |
| iii pubsub | Let other iii tools subscribe to bot events | `LOG_TO_PUBSUB=true`, `LOG_PUBSUB_TOPIC=bot.logs` |
| WebSocket | Let external dashboards/tools watch logs live | `LOG_WEBSOCKET_ENABLED=true`, `LOG_WEBSOCKET_PORT=3120` |

### Events visible in terminal

When running the bot in the CLI, the terminal should show human-readable events such as:

```text
[telegram.message.received] chat=123 user=456 msg=77 text="hello"
[l0.add.success] conversation=telegram:123 role=user id=...
[ai.request.start] provider=anthropic model=claude-sonnet-4-6 history_user=15 history_assistant=15
[tool.start] trace=... tool=l0_search query="editor" limit=10
[tool.success] trace=... tool=l0_search results=1 latency_ms=24
[ai.response.success] provider=anthropic latency_ms=1200 text="You like Helix."
[telegram.message.sent] chat=123 reply_to=77
```

Terminal logs may include short sanitized message snippets so development is easy. Full raw messages should still be written to L0/database, not only terminal output.

### Structured log event schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotLogEvent {
    pub id: String,
    pub timestamp_ms: i64,
    pub level: LogLevel,
    pub event: String,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub conversation_id: Option<String>,
    pub telegram_chat_id: Option<i64>,
    pub telegram_user_id: Option<u64>,
    pub tool_name: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub message: String,
    pub fields: serde_json::Value,
}
```

### JSONL persistence

Every structured event should be appended to the configured JSONL file:

```jsonl
{"timestamp_ms":1780000000000,"level":"info","event":"telegram.message.received","conversation_id":"telegram:123","fields":{"message_id":77}}
{"timestamp_ms":1780000000200,"level":"info","event":"tool.success","trace_id":"...","tool_name":"l0_search","fields":{"latency_ms":24,"results":1}}
```

Requirements:

- Create parent log directory if missing.
- Append one JSON object per line.
- Flush regularly so logs remain useful during crashes.
- Rotate later if file becomes too large; not required in v1.

### Database/L0 logging

Important events should also be written to durable storage:

- Telegram message received/sent
- L0 add/list/search operations
- AI request/response metadata
- Tool call start/success/failure
- Health status transitions

Full user and assistant message content belongs in L0 records. Observability records should usually store metadata and sanitized snippets only.

### Pubsub logging

Publish structured log events to iii pubsub so other tools can subscribe.

```text
topic = bot.logs
payload = BotLogEvent JSON
```

Use pubsub for live observability, not as the only durable store. If pubsub publish fails, log the failure locally but do not fail the bot request.

### WebSocket log stream

Create a local WebSocket server that broadcasts `BotLogEvent` JSON to connected clients.

Default:

```text
ws://127.0.0.1:3120/logs
```

Behavior:

- Each connected client receives new log events.
- The server may send the latest health report on connect.
- WebSocket failures must not break Telegram or AI handling.
- Bind to localhost by default for safety.

Suggested component:

```rust
pub struct LoggingBus {
    sinks: Vec<Arc<dyn LogSink + Send + Sync>>,
}

#[async_trait]
pub trait LogSink {
    async fn emit(&self, event: BotLogEvent) -> Result<()>;
}
```

## Tool lifecycle hooks

All custom AI tools should execute through a wrapper that runs lifecycle hooks around the actual tool function. This keeps injection, logging, auditing, and failure handling consistent across `l0_add`, `l0_search`, and `l0_list`.

The hook design should be internal to this bot and provider-neutral. If `aisdk` exposes native tool middleware/lifecycle APIs, use them. If not, implement the hooks in the Rust wrapper functions that the `#[tool]` macro calls.

### Hook flow

```text
AI requests tool call
    |
    v
pre_tool_use
    |
    +--> inject conversation_id, telegram metadata, request_id, trace_id
    +--> enforce policy/scope checks
    +--> write optional L0 tool_call audit record
    |
    v
actual tool implementation
    |
    +--> success --> post_tool_use
    |                  +--> log success metadata
    |                  +--> write L0 tool_result audit record
    |                  +--> return tool JSON to model
    |
    +--> failure --> post_tool_failure
                       +--> log error metadata
                       +--> write L0 tool_failure audit record
                       +--> return structured tool error JSON to model
                       +--> AI may retry with another tool call until failure budget is exhausted
```

### Tool failure retry budget

After `post_tool_failure`, the AI may try another tool call or retry with corrected arguments, but the bot must enforce a hard maximum of 5 failed tool calls per AI request.

Default:

```env
MAX_TOOL_FAILURE_RETRIES=5
```

Rules:

- Count every failed custom tool invocation in the current AI request.
- If failures are `< 5`, return the sanitized tool error payload to the model and allow the agent loop to continue.
- If failures reach `5`, stop the tool loop and return a final error to Telegram.
- Log `tool.failure_budget_exceeded` to terminal, JSONL, database/L0, pubsub, and WebSocket.
- Do not allow infinite tool-call loops.

User-facing final error:

```text
Tool execution failed too many times. Please try again or rephrase your request.
```

Suggested state:

```rust
#[derive(Debug, Clone)]
pub struct ToolLoopBudget {
    pub max_failures: usize,
    pub failures: usize,
}

impl ToolLoopBudget {
    pub fn record_failure(&mut self) -> bool {
        self.failures += 1;
        self.failures < self.max_failures
    }
}
```

### Tool context injected by `pre_tool_use`

```rust
#[derive(Debug, Clone)]
pub struct ToolRuntimeContext {
    pub request_id: String,
    pub trace_id: String,
    pub conversation_id: String,
    pub telegram_chat_id: i64,
    pub telegram_user_id: Option<u64>,
    pub telegram_message_id: Option<i32>,
    pub tool_name: String,
    pub started_at_ms: i64,
}
```

`pre_tool_use` is responsible for producing this context. Tool implementations should not trust model-provided scope fields for `conversation_id` or Telegram IDs. Those values must come from the bot runtime.

### `pre_tool_use`

Responsibilities:

- Inject trusted runtime context into the tool call.
- Attach `conversation_id`, Telegram metadata, `request_id`, and `trace_id`.
- Enforce tool scope:
  - L0 tools can only access the current conversation by default.
  - Cross-chat access is denied unless a future admin-only policy explicitly allows it.
- Clamp limits such as `limit <= 50`.
- Reject empty or overlong inputs.
- Optionally write an L0 audit record with `source = ToolCall`.

Suggested signature:

```rust
pub async fn pre_tool_use(
    tool_name: &str,
    raw_args: serde_json::Value,
    runtime: TelegramMeta,
    l0: Arc<dyn L0Repository + Send + Sync>,
) -> Result<ToolRuntimeContext>;
```

### `post_tool_use`

Responsibilities:

- Log a success event with metadata only.
- Record duration/latency.
- Store a compact L0 `ToolResult` audit record when `TOOL_AUDIT_LOG_TO_L0=true`.
- Return the successful tool result to the model.

Do not log full raw user content by default. Prefer counts, IDs, status, and small sanitized snippets.

Suggested signature:

```rust
pub async fn post_tool_use(
    ctx: &ToolRuntimeContext,
    result: &serde_json::Value,
    l0: Arc<dyn L0Repository + Send + Sync>,
) -> Result<()>;
```

### `post_tool_failure`

Responsibilities:

- Log a failure event with metadata and error class.
- Store a compact L0 `ToolResult` or dedicated failure audit record.
- Return a structured, model-readable error payload.
- Avoid exposing internal stack traces, secrets, or database paths to the model.

Suggested tool error payload:

```json
{
  "ok": false,
  "error": {
    "code": "l0_search_failed",
    "message": "The L0 search tool failed. Try again later."
  }
}
```

Suggested signature:

```rust
pub async fn post_tool_failure(
    ctx: &ToolRuntimeContext,
    error: &anyhow::Error,
    l0: Arc<dyn L0Repository + Send + Sync>,
) -> serde_json::Value;
```

### Tool wrapper pattern

Each tool should follow this pattern:

```rust
pub async fn run_l0_search_tool(
    input: L0SearchInput,
    runtime: TelegramMeta,
    l0: Arc<dyn L0Repository + Send + Sync>,
) -> serde_json::Value {
    let ctx = match pre_tool_use("l0_search", serde_json::json!(input), runtime, l0.clone()).await {
        Ok(ctx) => ctx,
        Err(error) => {
            return serde_json::json!({
                "ok": false,
                "error": {
                    "code": "pre_tool_use_failed",
                    "message": error.to_string()
                }
            });
        }
    };

    match l0.search(&ctx.conversation_id, &input.query, input.limit.unwrap_or(10) as usize).await {
        Ok(records) => {
            let result = serde_json::json!({ "ok": true, "results": records });
            let _ = post_tool_use(&ctx, &result, l0.clone()).await;
            result
        }
        Err(error) => post_tool_failure(&ctx, &error, l0.clone()).await,
    }
}
```

## Error handling

### iii/L0 errors

- If user-message L0 write fails, reply with a short memory-backend error.
- If assistant-response L0 write fails, still send the Telegram reply but log the error.
- If a tool search/list fails, return a structured tool error JSON to the model.
- Health monitor failures should update `HealthReport` and log status transitions.

### Tool hook errors

- If `pre_tool_use` fails, do not execute the tool implementation.
- If `post_tool_use` fails to write an audit record, do not fail the user-facing AI response; log the audit failure.
- If `post_tool_failure` cannot write an audit record, still return a sanitized structured error payload to the model.
- Hook logs must include `request_id`, `trace_id`, `conversation_id`, `tool_name`, status, and latency when available.

### AI timeout errors

- Each AI request attempt has `AI_AGENT_TIMEOUT_SECS`.
- Timeout attempts are retried up to `AI_AGENT_MAX_TIMEOUT_RETRIES=3` total attempts.
- Log every timeout attempt as `ai.request.timeout`.
- Log final timeout exhaustion as `ai.request.timeout_exhausted`.
- Do not retry forever.
- If all attempts time out, reply to Telegram:

```text
The AI request timed out after several attempts. Please try again.
```

### AI provider errors

Reply to Telegram:

```text
AI provider error. Please try again.
```

Log without secrets:

- provider
- model
- conversation_id
- error chain

Never log API keys or full credentials.

### Structured output errors

- Retry once with stricter JSON-only instruction.
- If still invalid:
  - for normal chat, fall back to raw text if available;
  - for command/action flows, return a parse error.

## Privacy and safety

Because L0 stores raw Telegram content:

- Do not log full user message content by default.
- Do not store API keys or credentials in L0.
- Scope memory by `conversation_id`.
- Do not allow cross-chat memory search without an explicit admin-only feature.
- Be conservative in group chats.
- Add future data deletion commands if this bot will be used by real users.

## Dependency updates

Current `Cargo.toml` should likely add:

```toml
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
async-trait = "0.1"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
pretty_env_logger = "0.5"
tokio-tungstenite = "0.26"
futures-util = "0.3"
```

Optional later:

```toml
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Implementation plan

### Milestone 1 — Bot skeleton

Files:

```text
src/main.rs
src/config.rs
src/telegram.rs
src/error.rs
```

Tasks:

- Load `.env`.
- Parse bot/AI/iii config.
- Initialize `teloxide::Bot`.
- Add dispatcher.
- Add `/start`, `/help`, `/model`, and `/health`.
- Add health monitor shared state with placeholder checks.
- Temporarily echo normal messages.

Validation:

```bash
cargo check
cargo run
```

Expected result:

- Bot starts.
- `/start` works.
- `/help` works.
- `/health` returns the latest health report.
- Normal text receives a basic response.

### Milestone 2 — L0 types and repository

Files:

```text
src/types.rs
src/l0/mod.rs
src/l0/model.rs
src/l0/repository.rs
src/l0/iii_repository.rs
src/l0/search.rs
```

Tasks:

- Add `L0Record`, `L0Role`, `L0Source`.
- Add `TelegramMeta`.
- Add `L0Repository` trait.
- Add iii-backed repository.
- Implement `add`.
- Implement `list`.
- Implement naive `search` through list + filter.

Validation:

- `/l0list` returns recent records.
- `/l0search <query>` returns matching records.
- Records survive bot restart if iii storage is persistent.

### Milestone 3 — Logging bus and live observability

Files:

```text
src/logging/mod.rs
src/logging/events.rs
src/logging/terminal.rs
src/logging/jsonl.rs
src/logging/pubsub.rs
src/logging/websocket.rs
src/logging/redaction.rs
```

Tasks:

- Add `BotLogEvent` and `LogSink` abstractions.
- Add terminal sink so CLI shows Telegram messages, AI calls, tool calls, and health transitions.
- Add JSONL sink at `./logs/bot-events.jsonl`.
- Add database/L0 logging for important audit events.
- Add iii pubsub sink on `bot.logs`.
- Add localhost WebSocket stream at `ws://127.0.0.1:3120/logs`.
- Add redaction helpers for secrets and long content.

Validation:

- `cargo run` shows readable event logs in terminal.
- JSONL file receives one valid JSON object per event.
- Pubsub subscribers can receive log events.
- WebSocket client can receive live log events.
- Logging failures do not break Telegram message handling.

### Milestone 4 — Health monitor hardening

Files:

```text
src/health/mod.rs
src/health/monitor.rs
src/health/checks.rs
src/health/model.rs
src/l0/mod.rs
src/telegram/handlers.rs
```

Tasks:

- Add `HealthReport`, `HealthStatus`, and `HealthCheck` types.
- Implement startup health checks for iii, SQLite/database, L0 write, and L0 read.
- Add periodic health checks using a Tokio background task.
- Cache latest health report in shared state.
- Wire `/health` to display the latest report.
- Log health status transitions.

Validation:

- Healthy database shows `healthy`.
- Stopped iii/database shows `unhealthy` or `degraded`.
- `/health` returns current statuses without exposing secrets.

### Milestone 5 — Basic AI chat

Files:

```text
src/agents/service.rs
src/agents/provider.rs
src/agents/history.rs
src/telegram/handlers.rs
```

Tasks:

- Add `AiService`.
- Add separated prompt files under `src/agents/prompts/`.
- Add prompt loader/composer in `src/agents/prompts.rs`.
- Select OpenAI or Anthropic from config.
- Convert recent L0 records to bounded `aisdk` message history.
- Enforce max 15 user messages and max 15 assistant messages per request.
- Wrap the AI agent call with timeout and retry helper.
- Enforce `AI_AGENT_TIMEOUT_SECS=60` and `AI_AGENT_MAX_TIMEOUT_RETRIES=3`.
- Call `.generate_text()`.
- Store assistant response as L0.
- Send response to Telegram.

Validation:

- User sends `hello`.
- Bot calls configured provider with no more than 15 user + 15 assistant history messages.
- Timed-out AI calls retry up to 3 attempts.
- Bot replies, or returns the timeout error after 3 timed-out attempts.
- L0 contains user and assistant records.

### Milestone 6 — L0 tools with lifecycle hooks

Files:

```text
src/agents/tools.rs
src/agents/tool_hooks.rs
src/agents/tool_loop.rs
src/agents/service.rs
src/l0/mod.rs
```

Tasks:

- Add `l0_add` tool.
- Add `l0_search` tool.
- Add `l0_list` tool.
- Add `pre_tool_use` hook for trusted runtime context injection and policy checks.
- Add `post_tool_use` hook for success logging and L0 audit records.
- Add `post_tool_failure` hook for sanitized failure logging and L0 audit records.
- Wrap every L0 tool with the lifecycle hook flow.
- Enforce `MAX_TOOL_FAILURE_RETRIES=5` per AI request.
- Stop the tool loop with a final error when the failure budget is exhausted.
- Register tools on AI request.
- Store tool calls, tool results, and tool failures in L0.

Manual validation:

```text
User: remember that my favorite editor is helix
Bot: Got it.
User: what editor do I like?
Bot: You like Helix.
```

Expected result:

- The AI can discover the memory through recent history or `l0_search`.
- Tool call/result/failure records are saved in L0.
- Hook logs include `request_id`, `trace_id`, `conversation_id`, `tool_name`, status, and latency.

### Milestone 7 — Structured output

Files:

```text
src/types.rs
src/agents/structured.rs
src/agents/service.rs
```

Tasks:

- Add `TelegramAssistantOutput`.
- Call `.schema::<TelegramAssistantOutput>()`.
- Parse with `.into_schema()`.
- Send `output.reply` to Telegram.
- Store structured metadata in L0 raw JSON.

Validation:

- Anthropic provider returns valid structured output.
- OpenAI provider returns valid structured output.
- Invalid output retry path works.

### Milestone 8 — SQLite/iii hardening

Tasks:

- Verify whether `iii-stream` can use the configured SQLite database directly.
- If yes, update `config.yaml` so L0 stream records are SQLite-backed.
- If no, keep iii stream as v1 and add a local SQLite-backed `L0Repository` implementation later.
- Add restart persistence checks.
- Ensure health checks report SQLite/L0 failures accurately.

Validation:

- Start iii.
- Start bot.
- Send messages.
- Restart bot and iii.
- `/l0list` still shows prior records.

## Testing plan

### Unit tests

- L0 record serialization/deserialization.
- Config parsing.
- L0 search filtering.
- L0-to-aisdk message conversion.
- Health report aggregation and status transitions.
- Bounded history selection: max 15 user + 15 assistant messages.
- Tool hook success/failure wrapper behavior.
- Tool failure budget enforcement after 5 failures.
- Structured log event serialization.
- Structured output schema generation.

### Integration tests

- iii add/list/search round trip.
- Health monitor against running and stopped iii/database services.
- Logging bus with terminal, JSONL, database/L0, pubsub, and WebSocket sinks.
- AI service with mock L0 repository.
- AI history request never exceeds 15 user + 15 assistant messages.
- AI timeout retry behavior with max 3 attempts.
- L0 tools with pre/post/failure hook audit records.
- L0 tool loop stops after 5 tool failures.
- Telegram handler logic with mocked dependencies where practical.

### Manual tests

```text
/start
/help
/model
/health
/l0list
/l0search hello
normal chat
memory recall
AI timeout retry max 3 attempts
tool success audit logging
tool failure audit logging
tool failure budget max 5
terminal logs visible while chatting
JSONL log persistence
pubsub log subscription
WebSocket log stream
structured output with Anthropic
structured output with OpenAI
restart persistence check
health status after iii/database outage
```

## Done criteria

The implementation is complete when:

1. `cargo check` passes.
2. Bot runs with `cargo run`.
3. Telegram `/start`, `/help`, `/model`, `/health`, `/l0list`, and `/l0search` work.
4. Normal Telegram messages produce AI replies.
5. User and assistant messages are stored as L0 records.
6. Recent L0 records are passed into `aisdk` as bounded history with max 15 user + 15 assistant messages.
7. Health monitor reports iii/database/L0 status and updates on failures.
8. Terminal logs show Telegram chat flow, L0 operations, AI calls, tool calls, and health transitions.
9. Logs are persisted to JSONL and database/L0.
10. Logs are published to iii pubsub and broadcast over the local WebSocket stream.
11. AI calls have timeout protection and retry up to 3 timed-out attempts.
12. AI tools can add, search, and list L0 records.
13. `pre_tool_use`, `post_tool_use`, and `post_tool_failure` hooks run for every custom tool.
14. Tool failures can be retried by the AI but stop with a final error after 5 failed tool calls.
15. Tool success and failure logs are written without exposing secrets.
16. Structured output works with the configured OpenAI and Anthropic providers.
17. L0 records persist across bot restarts.
