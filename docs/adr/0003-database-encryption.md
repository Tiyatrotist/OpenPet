# ADR 0003: Encrypted Local Storage & Windows Credential Isolation

## Status
Accepted

## Context
OpenPet stores persistent user context: settings, reminder schedules, conversational facts, and installed pet metadata. Because reminders and facts may contain personal information, storing plaintext SQLite files on disk introduces local privacy risks from malware or unauthorized local accounts.

Furthermore, API keys for optional AI providers (OpenAI, Anthropic, Gemini) must never be stored alongside application data or serialized into log outputs.

## Decision
1. Application metadata and structured facts are managed in SQLite via `openpet-storage` with transactional schema migrations.
2. All sensitive cryptographic keys and third-party API tokens are strictly isolated in the Windows Credential Manager (`CredWriteW` / `CredReadW`) under user-specific targets (`OpenPet/Credentials/<Kind>`).
3. Memory buffers containing credentials implement `zeroize::ZeroizeOnDrop` via `SecretString`, and their `Display` / `Debug` traits strictly render `[REDACTED_SECRET]`.
4. API keys are never relayed across the IPC pipe to Control Center. The UI receives only boolean status flags (`has_credential = true`).

## Consequences
### Positive
- Defense-in-depth against credential harvesting.
- Zero credential leakage in application log files or crash dumps.
- Clean migration path to SQLCipher for full at-rest encryption.

### Negative
- Windows Credential Manager dependency requires mock/memory vault fallback during non-interactive automated test execution.
