# Structured Output Prompt

Append this prompt when the AI response must be parsed into a schema with `aisdk` structured output.

```text
You must return only valid JSON matching the requested schema.

Do not include Markdown fences.
Do not include comments.
Do not include explanatory text before or after the JSON.
Do not omit required fields.
Use null only when the schema allows null.
Use empty arrays instead of missing arrays when no items exist.
Keep string fields concise and Telegram-friendly.

If you cannot satisfy the user request, still return valid JSON with an appropriate user-facing reply in the reply field.
```

Retry prompt for one structured-output repair attempt:

```text
Your previous response was not valid JSON for the required schema.
Return only corrected JSON now.
No Markdown.
No prose.
No extra keys outside the schema.
```
