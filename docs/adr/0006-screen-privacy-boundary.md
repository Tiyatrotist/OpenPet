# ADR 0006: Local-Only Screen Awareness & Ironclad Privacy Boundary

## Status
Accepted

## Context
Desktop companion applications that react to user activity often capture screens continuously and upload frames or OCR transcriptions to remote cloud services. This introduces massive privacy hazards (leaking passwords, personal emails, medical records, or confidential work documents).

## Decision
1. Screen Awareness is disabled by default (`screen_analysis_enabled = false`).
2. When disabled, the Windows Graphics Capture API initialization count is strictly zero.
3. Raw frame buffers never cross the crate boundary (`openpet-screen`). They are immediately processed in local memory into high-level categorical signals (`Idle`, `Writing`, `Meeting`, `Video`, `Unknown`) and discarded.
4. Raw screenshots and OCR transcriptions are never written to disk, never logged, never transmitted across network, never forwarded to AI providers, never sent over IPC, and never accessible to plugins.
5. Instant Privacy Mode: A single toggle from the system tray or Control Center halts capture immediately and flushes transient buffers.

## Consequences
### Positive
- Verifiable privacy architecture: Automated tests assert zero capture calls when disabled.
- Full user trust: Pet behavior can react to general activity context without spying.

### Negative
- Advanced multi-modal visual reasoning over screen pixels is deliberately omitted to preserve user privacy.
