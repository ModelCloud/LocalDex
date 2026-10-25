# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Lifecycle hooks

Admins can set top-level `allow_managed_hooks_only = true` in
`requirements.toml` to ignore user, project, and session hook configs while
still allowing managed hooks from requirements and managed config layers. This
setting is only supported in `requirements.toml`; putting it in `config.toml`
does not enable managed-hooks-only mode.

## LocalDex providers

LocalDex registers an `openrouter` provider by default: it points at
`https://us.openrouter.ai/api/v1` and reads its credential from the
`OPENROUTER_API_KEY` environment variable (or `env_key_fallbacks`/`auth.json`,
like every other `env_key` provider). No keys are stored in the repo.
The US regional endpoint requires an OpenRouter Business or Enterprise plan.

OpenRouter provider routing is configured through the `provider` table on the
provider entry and is forwarded verbatim as the request's top-level `provider`
object:

```toml
model_provider = "openrouter"
model = "deepseek/deepseek-v4.1-flash-1017"

[model_providers.openrouter.provider]
only = ["wafer/mxfp4"]
allow_fallbacks = false
```

The bundled catalog already includes `deepseek/deepseek-v4.1-flash-1017`
(context window, reasoning levels, and its DeepSeek-tuned system prompt), so
models served through OpenRouter work without a remote model catalog.

Selecting this slug chooses the built-in `openrouter` provider at startup or
during a live model switch. Selecting `DS v4.1 Flash 1017` chooses the local
provider; selecting a bundled OpenAI model from OpenRouter returns to OpenAI.
Managed provider restrictions still apply.

Both DS entries default to Max reasoning and use the same prompt adapted from
DeepSeek Harness's `snapshots/session/worktree/system-prompt.expected.md`.
The adaptation replaces Harness-only file, background-job, goal, and delegation
tools with LocalDex equivalents; it is not a byte-for-byte copy. Explicit
user-configured instructions can still override the model's default prompt.
