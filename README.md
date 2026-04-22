# Kernelaw

Kernelaw is a hackable, modular agent kernel for building extensible multi-agent platforms.

## Design Draft

- Current kernel design draft (CN): [`docs/current-kernel-design.md`](docs/current-kernel-design.md)

## Kernel Core Bootstrap

This repository now includes a first Rust workspace bootstrap for `kernel-core`:

- core runtime types (`Process`, `Event`, `TapeItem`, `Effect`)
- `ModelConfig` validation for provider/model settings
- a minimal `step()` runtime skeleton (`project -> deliberate -> execute -> commit`)
- unit tests with a mock provider

### Quick start

```bash
cargo test
```

### Local model test option

If you want to test against a tiny local model, you can run one outside this repo (for example via Ollama on `http://127.0.0.1:11434`) and then wire its endpoint/model into `ModelConfig` in integration tests or examples.


## Current parity progress (Hermes/OpenClaw inspired)

Implemented in `kernel-core`:

- typed hook lifecycle runner (`before_reply` and full stage enum scaffold)
- module registry with kinds covering provider/tool/memory/policy/channel/observer/webui/gateway-ws/browser/mcp
- capability gate in runtime (e.g. `CallTool` requires `tool.call`)
- Ollama-compatible provider adapter (`/api/chat`) and static provider for local tests
- tool runtime registry/executor scaffold with `tool.result` tape projection

Not yet complete:

- full Hermes-level provider/tool compatibility layer
- full OpenClaw-level gateway/hub/channel delivery implementation

These are the next implementation milestones.
