# Telegram AI L0 Bot

A Rust Telegram AI assistant that combines Telegram chat handling, AI replies through `aisdk`, raw L0 memory, health checks, and structured logging through terminal, JSONL, and iii pubsub.

## Features

- Telegram bot powered by `teloxide`
- AI provider support through `aisdk`
  - Anthropic
  - OpenAI
  - OpenAI-compatible endpoints
- Raw L0 memory backend through iii streams or in-memory mode
- Read-only AI memory tools:
  - `l0_list`
  - `l0_search`
- Structured prompt composition hardcoded in Rust
- Telegram inline menu buttons
- Health checks for bot process ping and iii pubsub
- Logging sinks:
  - terminal
  - date-prefixed JSONL files
  - iii pubsub

## Architecture flow

```text
Telegram User
    |
    v
teloxide Bot
    |
    v
Telegram Dispatcher
    |
    +--> Command handler
    |       |
    |       +--> /start and /menu
    |       |       +--> send inline menu buttons: Help, Health
    |       |
    |       +--> /help
    |       |       +--> send command help text
    |       |
    |       +--> /health
    |       |       +--> read latest HealthReport
    |       |       +--> show ping and iii_pubsub checks
    |       |
    |       +--> /l0list and /l0search
    |               +--> read raw L0 records for current Telegram conversation
    |
    +--> Callback query handler
    |       |
    |       +--> Help button
    |       |       +--> send new help message
    |       |
    |       +--> Health button
    |               +--> send new health message
    |
    +--> Text message handler
            |
            +--> L0 repository: store raw user Telegram message
            |
            +--> L0 repository: load bounded recent conversation history
            |
            +--> agents::history: adapt L0 records into aisdk messages
            |
            +--> agents::prompts: compose hardcoded system prompt
            |
            +--> aisdk LanguageModelRequest
            |       |
            |       +--> Anthropic, OpenAI, or OpenAI-compatible provider
            |       |
            |       +--> read-only L0 tools
            |               |
            |               +--> l0_list: recent raw records
            |               +--> l0_search: older or specific raw records
            |               +--> tool hooks: audit start/success/failure to logs and L0
            |
            +--> L0 repository: store assistant response
            |
            +--> Telegram send_message()

Runtime services
    |
    +--> HealthMonitor
    |       +--> ping
    |       +--> iii_pubsub
    |
    +--> LoggingBus
            +--> terminal
            +--> date-prefixed JSONL file
            +--> iii pubsub
```

## Telegram commands

| Command | Description |
| --- | --- |
| `/start` | Show the button menu |
| `/menu` | Show the button menu |
| `/help` | Show command help text |
| `/model` | Show current AI provider and model |
| `/health` | Show ping and iii pubsub health |
| `/l0list` | List recent L0 records |
| `/l0search <query>` | Search L0 memory |

## Menu buttons

`/start` and `/menu` show an inline menu:

```text
Menu
Choose an option:

[ Help ] [ Health ]
```

Button behavior:

- `Help` sends a new help message.
- `Health` sends a new health message.

Direct commands still work:

- `/help`
- `/health`

## Health checks

`/health` reports only runtime/backend checks:

```text
Health: healthy
- ping: healthy (0ms)
- iii_pubsub: healthy (12ms)
```

Checks:

- `ping` — verifies the bot process/health monitor is alive.
- `iii_pubsub` — publishes a small health event through iii pubsub using `III_URL`, `LOG_PUBSUB_TOPIC`, and `DB_HEALTH_TIMEOUT_MS`.

The health output intentionally does not show:

- `ai_provider_config`
- `telegram_config`
- `l0_round_trip`

## Logging

Supported logging outputs:

- terminal
- JSONL file
- iii pubsub

WebSocket logging has been removed. Use pubsub for streamed log consumption.

### JSONL filenames

`LOG_JSONL_PATH` is configured as a base path internally, and the sink prefixes the filename with the UTC date.

Example configured/default base path:

```text
./logs/bot-events.jsonl
```

Actual file:

```text
./logs/YYYY-MM-DD-bot-events.jsonl
```

Example:

```text
./logs/2026-06-01-bot-events.jsonl
```

## Setup

Copy the example environment file:

```bash
cp .env.example .env
```

Fill in required values:

```env
TELOXIDE_TOKEN=
III_URL=ws://127.0.0.1:49134
AI_PROVIDER=anthropic
AI_MODEL=claude-sonnet-4-6
ANTHROPIC_API_KEY=
OPENAI_API_KEY=
```

Start iii separately if using the iii-backed L0 repository/pubsub:

```bash
iii --config config.yaml
```

Run the bot:

```bash
cargo run
```

If `TELOXIDE_TOKEN` is not set, the Telegram dispatcher will not start.

## Configuration

See `.env.example` for the full list.

Common settings:

```env
TELOXIDE_TOKEN=
III_URL=ws://127.0.0.1:49134
AI_PROVIDER=anthropic
AI_MODEL=claude-sonnet-4-6
ANTHROPIC_API_KEY=
OPENAI_API_KEY=

L0_HISTORY_LIMIT=30
L0_MAX_USER_HISTORY=15
L0_MAX_ASSISTANT_HISTORY=15
L0_SEARCH_LIMIT=10

HEALTH_CHECK_INTERVAL_SECS=60
DB_HEALTH_TIMEOUT_MS=2000

LOG_LEVEL=info
LOG_TO_TERMINAL=true
LOG_TO_JSONL=true
LOG_TO_DATABASE=true
LOG_TO_PUBSUB=true
LOG_PUBSUB_TOPIC=bot.logs
```

### AI providers

Supported `AI_PROVIDER` values:

- `anthropic`
- `claude`
- `openai`
- `open_ai`
- `openai_compatible`
- `openai-compatible`
- `open_ai_compatible`
- `compatible`

Optional endpoint overrides:

```env
OPENAI_BASE_URL=
OPENAI_API_PATH=
ANTHROPIC_BASE_URL=
ANTHROPIC_API_PATH=
```

## L0 memory

L0 memory is raw event history. It may include:

- Telegram user messages
- assistant responses
- tool calls
- tool results
- tool failures
- health events
- logging/audit events

The AI can read L0 memory through:

- `l0_list` — recent records
- `l0_search` — older or specific records

The AI cannot call `l0_add`. L0 records are stored automatically by the runtime.

For local in-memory L0 instead of iii-backed L0, set:

```bash
L0_USE_MEMORY=1 cargo run
```

## Development

Run checks:

```bash
cargo check
```

Run tests:

```bash
cargo test
```

Run a targeted test:

```bash
cargo test telegram::format::tests::formats_menu_keyboard_with_help_and_health_buttons
```

## Project structure

```text
src/
  agents/      AI provider, prompts, structured output, tools, tool loop
  health/      Health model, checks, monitor
  l0/          L0 model and repositories
  logging/     Terminal, JSONL, pubsub logging
  telegram/    Commands, dispatcher, formatters, handlers
  config.rs    Environment configuration
  main.rs      Application wiring
```

## Notes

- Prompt bodies are hardcoded in `src/agents/prompts.rs`.
- Runtime data and logs are not source-of-truth docs:
  - `data/stream_store/**`
  - `logs/*.jsonl`
- Do not commit real `.env` secrets.
