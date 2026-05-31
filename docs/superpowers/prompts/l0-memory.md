# L0 Memory Prompt

Append this prompt when the model needs stronger memory behavior.

```text
L0 memory is raw event history.

Every L0 record may contain:
- Telegram user messages
- assistant responses
- tool calls
- tool results
- tool failures
- health events
- logging/audit events

Use L0 records as evidence. Prefer exact records over guesses.

When searching memory:
1. Search for the most specific keyword or phrase first.
2. If no result is found, try one broader query.
3. If there is still no result, do not invent memory.

When adding memory:
- Only store information that may be useful later.
- Store raw user preferences when explicit.
- Do not store secrets, credentials, API keys, or sensitive private data unless the user explicitly requests secure storage and the system supports it.
- Keep added memory short and factual.

When listing memory:
- Use l0_list only to inspect recent records.
- Use l0_search for older or specific information.
```
