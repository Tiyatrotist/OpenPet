# OpenPet OSS Maintenance Evidence

## 1. Project Identity & Governance
- **Repository**: OpenPet
- **Owner / Public Maintainer Identity**: Tiyatrotist
- **License**: GNU Affero General Public License v3.0 (AGPL-3.0)
- **Primary Runtime Target**: Windows 11 x64 (Rust stable)

## 2. Engineering Verification Snapshot
- **Test Suite Results**: 34 unit and contract tests executed across 18 crates, 0 failures.
  - Contract testing for AI providers (streaming, 401 auth, rate limit exponential backoff with jitter).
  - Security tests for PetPack archive parsing (Zip-Slip rejection, malicious archive rejection, atomic installation).
  - Privacy regression tests (Asserting zero capture API calls when screen analysis is disabled, instant privacy cutoff).
  - Utility AI simulation tests (Deterministic state transitions and drag coordinate invariance).
  - Plugin fault isolation tests (Fuel budget exhaustion disabling plugin without affecting host, least privilege denial).
  - Memory facts search and structured retrieval tests.
  - Confirmed tool call gating tests (Unconfirmed LLM tool execution rejected).
- **Code Quality Gates**:
  - `cargo clippy --workspace --all-targets`: Clean (0 warnings).
  - `cargo fmt --check`: Clean (100% formatted).
  - `cargo check --workspace`: Clean.

## 3. Privacy & Security Verifications
- **No Tracked Secrets**: Clean tree, all credentials stored exclusively in the Windows Credential Manager.
- **Redaction**: All secret memory types implement zeroize and custom `Debug`/`Display` implementations formatting strictly `[REDACTED_SECRET]`.
- **Zero Telemetry**: No third-party analytics SDKs, no telemetry reporting, no background beaconing.
