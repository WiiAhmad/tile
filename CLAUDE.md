# Project Instructions for Claude

This repository is a Rust Telegram AI bot built with `teloxide`, `aisdk`, and `rusqlite`.

## Core workflow

- Prefer small, focused changes that match the existing Rust style.
- Run targeted tests for changed modules, then `cargo test` before reporting completion.
- Run `cargo check` when changing types, config structs, dependencies, or module wiring.
- Do not commit, push, or open PRs unless explicitly asked.
- Do not edit runtime log files unless explicitly asked:
  - `logs/*.jsonl`

## Runtime architecture

- `src/main.rs` wires configuration, logging sinks, health monitor, L0 repository, AI service, and Telegram dispatcher.
- `src/config.rs` owns environment configuration.
- `src/telegram/**` owns Telegram commands, dispatcher routing, formatting, and handlers.
- `src/agents/**` owns AI prompt composition, provider selection, structured output, and AI-visible tools.
- `src/l0/**` owns raw L0 memory persistence/search through SQLite and FTS.
- `src/logging/**` owns terminal, mandatory JSONL, and SQLite database logging.
- `src/health/**` owns `/health` ping checks and health report modeling.

## Current behavior to preserve

### Prompts

- Prompt bodies are hardcoded in `src/agents/prompts.rs`.
- Do not recreate `src/agents/prompts/*.txt` prompt files.
- Keep prompt tests focused on key behavior, not full prompt snapshots.

### L0 tools exposed to AI

- AI-visible L0 tools are read-only:
  - `l0_search`
  - `l0_list`
- Do not expose `l0_add` to the AI. L0 records are stored automatically from user messages, assistant responses, and tool activity.
- Existing commented `l0_add` reference code in `src/agents/tools.rs` may remain as reference unless the user asks to remove it.

### Telegram commands

- `/start` sends the inline button menu.
- `/menu` sends the same inline button menu.
- `/help` sends help text directly.
- `/health` sends health text directly.
- Menu buttons send new messages rather than editing the menu message:
  - Help button -> help text
  - Health button -> latest health report

### Health checks

- `/health` should show only:
  - `ping`
- Do not re-add AI provider config, Telegram config, or L0 round-trip checks to `/health` unless the user asks.
- Health runs every 60 seconds with no env var for the interval.

### Logging

- Supported logging sinks:
  - terminal, always enabled
  - date-prefixed JSONL, always enabled
  - SQLite database logging, always enabled
- JSONL filenames are date-prefixed using UTC date only:
  - hard-coded `./logs/bot-events.jsonl`
  - actual `./logs/YYYY-MM-DD-bot-events.jsonl`
- Database log events are stored in `bot_log_events` inside `./data/database.db`.

### SQLite L0 storage

- L0 data is stored in the hard-coded SQLite database path `./data/database.db`.
- L0 writes and database log writes share one SQLite store/connection in the bot process.
- L0 list uses an ordered SQL query scoped by `conversation_id`.
- L0 search returns exact SQL substring matches first, then SQLite FTS5 matches, deduplicated by record id.

## Useful commands

```bash
cargo check
cargo test
cargo test <module>::<test_name>
```

Run the bot locally:
MUST ASK USER IF WANT TO RUN THIS
```bash
cargo run
```

If Telegram credentials are missing, the dispatcher will not start.

## Environment

Use `.env.example` as the template. Do not include real secrets in docs or commits.

Important env vars:

- `BOT_TOKEN`
- `AI_PROVIDER`
- `AI_MODEL`
- `AI_API_KEY`
- `AI_BASE_URL`
- `AI_API_PATH`

## Testing notes

- Tests are unit tests inside `src/main.rs` and module files.
- When changing config struct fields, update all test config builders in:
  - `src/agents/provider.rs`
  - `src/agents/service.rs`
  - `src/health/monitor.rs`
- When changing command enum variants, update handler match arms and command-name logging.
