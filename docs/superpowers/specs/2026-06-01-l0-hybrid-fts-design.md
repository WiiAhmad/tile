# L0 Hybrid FTS Design

## Goal

Upgrade L0 search from exact substring-only matching to a hybrid search path that keeps the existing substring behavior while adding SQLite FTS-ranked keyword search behind iii worker functions. The feature must apply to both Telegram `/l0search` and the AI-visible `l0_search` tool without exposing write access to the AI.

## Current state

- `L0Repository` exposes `add`, `list`, and `search`.
- `IiiL0Repository` currently writes raw records through `stream::set`, lists through `stream::list`, and implements search by listing all records for a conversation then calling local Rust substring search.
- `MemoryL0Repository` has the same substring behavior and is useful for tests and degraded/local runs.
- `l0_search` is read-only and takes `{ query, limit }`; Telegram `/l0search <query>` calls the same repository method.
- L0 search intentionally searches only `L0Record.content`, not `raw_json` or metadata.

## Desired behavior

Search remains scoped to the current Telegram conversation and still searches only `L0Record.content`.

Hybrid search combines two signals:

1. **Exact substring compatibility** — the current behavior remains valid. If a record content contains the query phrase case-insensitively, it should match.
2. **SQLite FTS keyword matching** — query terms can match content even when words are in a different order or separated by other words.

The public user interfaces stay simple:

- Telegram keeps `/l0search <query>`.
- The AI tool keeps `l0_search` with `query` and optional `limit`.
- No `l0_add` tool is exposed.

## Architecture

Add a Rust iii worker binary that registers L0 functions:

- `l0::add`
- `l0::list`
- `l0::search`

The worker owns the hybrid backend behavior. The Telegram bot remains a client of iii.

Data flow for writes:

```text
Bot IiiL0Repository.add(record)
  -> iii trigger l0::add
    -> stream::set raw record into telegram_l0
    -> upsert record into SQLite l0_records
    -> update SQLite FTS index
```

Data flow for search:

```text
Bot IiiL0Repository.search(conversation_id, query, limit)
  -> iii trigger l0::search
    -> exact substring query over SQLite l0_records.content
    -> SQLite FTS query over l0_records_fts.content
    -> merge and dedupe by L0 record id
    -> exact substring hits first, then FTS-ranked hits
    -> return at most limit L0Record values
```

`iii-stream` remains the raw event log. SQLite is the searchable index and can also serve `l0::list` once records have been indexed. This avoids adding a separate local SQLite store inside the Telegram bot process.

## SQLite schema

Use the configured SQLite file for L0 FTS, defaulting to `./data/iii.db` through `L0_FTS_SQLITE_PATH`.

Tables:

- `l0_records` stores one row per L0 record plus a `record_json` column containing the full serialized `L0Record`.
- `l0_records_fts` is an FTS5 virtual table over `content` using `l0_records` as external content.
- Triggers keep the FTS table synchronized on insert, update, and delete.

The FTS table indexes only `content`; `conversation_id` stays in `l0_records` and is applied as a filter when joining back to records.

## Fallback and compatibility

The bot should be able to run before the worker is enabled. Add config to control whether `IiiL0Repository` prefers custom L0 worker functions.

- When worker mode is disabled, behavior is unchanged: `stream::set`, `stream::list`, local substring search.
- When worker mode is enabled, `IiiL0Repository` calls `l0::add`, `l0::list`, and `l0::search`.
- If `l0::search` fails while worker mode is enabled, fall back to current `stream::list` plus substring search and return that result rather than breaking Telegram/AI responses.
- If `l0::add` fails while worker mode is enabled, fall back to `stream::set` so L0 raw history is not lost.

## Tool and Telegram behavior

Both existing entry points use `L0Repository::search`, so the feature is mostly transparent:

- `/l0search helix config` benefits from hybrid search.
- AI `l0_search` benefits from hybrid search.
- Tool audit remains unchanged: tool calls/results are written to L0, but searches still only match their short `content` strings.

Update the `l0_search` tool description and L0 prompt text to say search is hybrid keyword/phrase search over current-conversation L0 content.

## Error handling

- Empty queries and zero limits still return no results.
- FTS query strings are sanitized into safe tokens before using `MATCH`; unsupported punctuation should not produce SQL syntax errors.
- SQLite or FTS failures inside `l0::search` should fall back to exact substring search inside the worker if possible.
- Bot-level worker trigger failures should fall back to the existing stream/list substring path.

## Testing strategy

Unit tests:

- Existing substring search tests continue to pass.
- SQLite FTS index can add and list records.
- Hybrid search matches exact phrases.
- Hybrid search matches words out of order.
- Exact phrase hits are returned before looser FTS hits.
- Empty query and zero limit return empty results.
- Tool tests verify `l0_search` still exposes only read behavior and returns repository search results.

Integration/manual checks:

- `cargo check`
- targeted L0 tests
- targeted agent tool tests
- `cargo test`
- optional manual iii check with worker enabled, only after asking before starting the bot/long-running app processes.

## Scope exclusions

- No embeddings or semantic memory.
- No search over `raw_json` or metadata.
- No user-facing search mode flags in the first version.
- No AI-visible write tool.
- No changes to `/health` output beyond existing `ping` and `iii_pubsub` policy.
