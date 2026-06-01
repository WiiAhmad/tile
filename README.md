# Telegram AI L0 Bot

A Rust Telegram AI assistant that combines Telegram chat handling, AI replies through `aisdk`, raw L0 memory, health checks, and structured logging through terminal, JSONL, and SQLite.

## Features

- Telegram bot powered by `teloxide`
- AI provider support through `aisdk`
  - Anthropic
  - OpenAI
  - OpenAI-compatible endpoints
- Raw L0 memory stored directly in SQLite at `./data/database.db`
- Hybrid L0 search:
  - exact SQL substring matches first
  - SQLite FTS5 keyword matches second
- Read-only AI memory tools:
  - `l0_list`
  - `l0_search`
- Structured prompt composition hardcoded in Rust
- Telegram inline menu buttons
- Ping-only health checks
- Logging sinks:
  - terminal, always enabled
  - date-prefixed JSONL at `./logs/YYYY-MM-DD-bot-events.jsonl`, always enabled
  - SQLite table `bot_log_events` in `./data/database.db`, always enabled

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
    |       |       +--> read latest ping HealthReport
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
            +--> SQLite L0 repository: store raw user Telegram message
            |
            +--> SQLite L0 repository: load bounded recent conversation history
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
            |               +--> l0_search: exact + FTS search
            |               +--> tool hooks: audit start/success/failure to logs and L0
            |
            +--> SQLite L0 repository: store assistant response
            |
            +--> Telegram send_message()

Runtime services
    |
    +--> HealthMonitor
    |       +--> ping every 60 seconds
    |
    +--> LoggingBus
            +--> terminal
            +--> date-prefixed JSONL file
            +--> SQLite bot_log_events table
```

## Telegram commands

| Command | Description |
| --- | --- |
| `/start` | Show the button menu |
| `/menu` | Show the button menu |
| `/help` | Show command help text |
| `/model` | Show current AI provider and model |
| `/health` | Show bot health |
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

`/health` reports only process liveness:

```text
Health: healthy
- ping: healthy (0ms)
```

The health output intentionally does not show:

- `ai_provider_config`
- `telegram_config`
- `l0_round_trip`

The periodic health monitor runs every 60 seconds with no environment variable for the interval.

## Logging

Supported logging outputs:

- terminal, always enabled
- JSONL file, always enabled
- SQLite database table `bot_log_events`, always enabled

### JSONL filenames

The JSONL base path is hardcoded to:

```text
./logs/bot-events.jsonl
```

The sink prefixes the filename with the UTC date. Actual file:

```text
./logs/YYYY-MM-DD-bot-events.jsonl
```

Example:

```text
./logs/2026-06-01-bot-events.jsonl
```

### Database log events

Database log events are stored in:

```text
./data/database.db
```

Table:

```text
bot_log_events
```

## Setup

Copy the example environment file:

```bash
cp .env.example .env
```

Fill in required values:

```env
BOT_TOKEN=
AI_PROVIDER=anthropic
AI_MODEL=claude-sonnet-4-6
AI_API_KEY=
```

Optional AI endpoint overrides:

```env
AI_BASE_URL=
AI_API_PATH=
```

Run the bot:

```bash
cargo run
```

If `BOT_TOKEN` is not set, the Telegram dispatcher will not start.

## Configuration

See `.env.example` for the full list.

Common settings:

```env
BOT_TOKEN=
AI_PROVIDER=anthropic
AI_MODEL=claude-sonnet-4-6
AI_BASE_URL=
AI_API_PATH=
AI_API_KEY=

L0_HISTORY_LIMIT=30
L0_MAX_USER_HISTORY=15
L0_MAX_ASSISTANT_HISTORY=15
L0_SEARCH_LIMIT=10

MAX_TOOL_FAILURE_RETRIES=5
AI_AGENT_TIMEOUT_SECS=60
AI_AGENT_MAX_TIMEOUT_RETRIES=3

LOG_LEVEL=info
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

`AI_BASE_URL`, `AI_API_PATH`, and `AI_API_KEY` apply to the selected provider.

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
- `l0_search` — exact SQL substring matches plus SQLite FTS5 matches

The AI cannot call `l0_add`. L0 records are stored automatically by the runtime.

For local in-memory L0 instead of SQLite-backed L0, set:

```bash
L0_USE_MEMORY=1 cargo run
```

## SQLite storage

The bot stores L0 records and database log events in the hardcoded file:

```text
./data/database.db
```

Tables:

```text
l0_records
l0_records_fts
bot_log_events
```

Writes are serialized through one shared SQLite store/connection in the bot process.

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
  health/      Health model, ping check, monitor
  l0/          L0 model, SQLite repository, FTS store
  logging/     Terminal, JSONL, SQLite database logging
  telegram/    Commands, dispatcher, formatters, handlers
  config.rs    Environment configuration
  main.rs      Application wiring
```

## Notes

- Prompt bodies are hardcoded in `src/agents/prompts.rs`.
- Runtime logs are not source-of-truth docs:
  - `logs/*.jsonl`
- Do not commit real `.env` secrets.
