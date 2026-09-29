# LocalDex upstream sync

This repository follows `openai/codex` main. Keep upstream implementations in
place when merging; carry LocalDex behavior as small, provider-specific changes.

Compatibility points to preserve:

- Official OpenAI models use their official provider. `QB/DSV4.1-Flash` uses
  the configured `localdex` provider, including after a model switch or resume.
- LocalDex reads its live context capacity and compacts at context minus 4,196
  tokens. A changed capacity can trigger compaction before the next request.
- Mid-turn LocalDex steer cancels only the active local stream and retains the
  turn. Upstream Responses Lite interruption still drains its response.
- A dropped response stream cancels its upstream request. Keep this behavior
  when upstream changes response stream ownership.
- Upstream Guardian context overflow recovery remains available alongside
  LocalDex context overflow recovery.

After merging upstream, inspect these paths first:

- `codex-rs/model-provider-info/` and `codex-rs/models-manager/` for provider
  routing, catalog metadata, and context limits.
- `codex-rs/core/src/session/turn.rs` and `codex-rs/core/src/client.rs` for
  stream cancellation, model switching, and compaction.
- `codex-rs/codex-api/src/common.rs` for response stream ownership.

Use upstream conflict hunks for unrelated features. Compile the CLI, app
server, core, and TUI, then run the focused LocalDex model, provider switch,
stream interruption, and compaction tests before publishing a sync.
