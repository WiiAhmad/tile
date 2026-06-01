ccs codex --dangerously-skip-permissions


iii --config config.yaml

## L0 hybrid FTS search

L0 search can run in two modes:

1. Default mode: the bot uses `iii-stream` directly and performs local case-insensitive substring search over `L0Record.content`.
2. Worker mode: the bot calls custom iii functions `l0::add`, `l0::list`, and `l0::search`. The worker stores raw L0 records in `iii-stream` and indexes `L0Record.content` in SQLite FTS.

Enable worker mode:

```dotenv
L0_USE_WORKER_FUNCTIONS=true
L0_FTS_SQLITE_PATH=./data/iii.db
```

Run the worker in a separate terminal after iii is running:

```bash
cargo run --bin l0_fts_worker
```

Run the bot separately after the worker is registered. If `l0::search` fails, the bot falls back to the existing `stream::list` + substring search path. Search remains scoped to the current Telegram conversation and only searches L0 `content`, not `raw_json` or metadata.
