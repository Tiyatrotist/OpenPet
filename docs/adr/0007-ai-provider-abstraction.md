# ADR 0007: AI Provider Abstraction, Tool Confirmation & Error Normalization

## Status
Accepted

## Context
Different LLM providers (OpenAI, Anthropic Claude, Google Gemini, local Ollama) have wildly divergent HTTP schemas, error responses, rate-limiting policies, and streaming wire protocols.

Furthermore, desktop companion safety mandates that no generative AI model can directly invoke destructive mutations (creating reminders, deleting memories, modifying system files) without explicit human confirmation.

## Decision
1. We establish a uniform `ChatProvider` trait exposing streaming, tool calling, and capability flags.
2. Error Normalization: All HTTP, rate-limit, and authentication failures are normalized into `ProviderError`.
3. Transient errors (rate limits, network blips, timeouts) undergo automatic exponential backoff with randomized jitter.
4. Tool Call Confirmation Gate: Any tool proposed by an LLM (`reminder.create`, `reminder.delete`, `memory.create`) generates a `ToolCallProposal`. The host refuses execution unless `user_confirmed == true`.
5. Offline Testing: A `FakeChatProvider` simulates success, streaming deltas, 401s, 429s, 500s, and connection drops, allowing 100% of provider contract tests to run deterministically in CI without external API keys.

## Consequences
### Positive
- Pluggable provider ecosystem without changing application logic.
- Complete safety against prompt injection attacks or hallucinated tool invocations.
- Deterministic, offline-friendly test suite.

### Negative
- Requires maintaining adapter compatibility across upstream provider API revisions.
