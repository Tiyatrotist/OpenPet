# OpenPet Threat Model & Security Posture

## 1. Overview
OpenPet is a local-first desktop pet companion platform running on Windows 11. This document defines the threat landscape, asset classifications, and defense-in-depth mitigations.

---

## 2. Threat Analysis by Attack Surface

### 2.1 PetPack Package Parser (`openpet-petpack`)
- **Asset**: Host filesystem, application code directories.
- **Threat**: Directory traversal, decompression bombs, remote code execution via malicious archives.
- **Attack Vector**: Specially crafted `.openpet` zip archive containing `../../Windows/System32` or nested 100GB compression bombs.
- **Mitigation**:
  - Rejection of non-normal path components (`..`, absolute paths, drive letters, NTFS streams `:`).
  - Strict limits: Package size <= 256 MB, uncompressed size <= 512 MB, entries <= 2000, asset <= 128 MB.
  - Staging directory isolation: extracted to temporary staging, validated, and atomically moved.
- **Residual Risk**: Zero-day decompression flaws in underlying zip library.
- **Automated Verification**: `openpet_petpack::tests::test_zip_slip_rejection` and `test_malicious_zip_slip_archive_rejected`.

### 2.2 Sandboxed Plugins (`openpet-plugin-host`)
- **Asset**: Host CPU, memory, persistent database, user credentials.
- **Threat**: Denial of service via infinite loops, memory exhaustion, unauthorized host API access.
- **Attack Vector**: Rogue WebAssembly component executing an infinite loop or attempting to read memory facts.
- **Mitigation**:
  - Default capability: `NONE`.
  - Fuel quota: Hard limit per execution.
  - Fault isolation: Crashing plugin is marked faulted and disabled; host continues undisturbed.
- **Residual Risk**: Wasm runtime side-channel timing attacks.
- **Automated Verification**: `openpet_plugin_host::tests::test_least_privilege_default_denial` and `test_fuel_exhaustion_isolates_fault`.

### 2.3 Generative AI Tool Execution (`openpet-reminders`)
- **Asset**: User reminders, schedule database, desktop notifications.
- **Threat**: Prompt injection hijacking model to delete reminders or schedule spam.
- **Attack Vector**: Malicious chat prompt leading model to emit `reminder.delete` or `reminder.create`.
- **Mitigation**:
  - LLM can only emit a proposal (`ToolCallProposal`).
  - Execution strictly gated behind `user_confirmed == true`.
- **Residual Risk**: Social engineering confusing user to click confirm.
- **Automated Verification**: `openpet_reminders::tests::test_unconfirmed_tool_call_fails`.

### 2.4 Inter-Process Communication (`openpet-ipc`)
- **Asset**: OpenPet Host control plane and database operations.
- **Threat**: Local privilege escalation, cross-user IPC injection.
- **Attack Vector**: Another local user process connecting to the named pipe and issuing `Shutdown` or deleting data.
- **Mitigation**:
  - Named pipe scoped to current user session (`\\.\pipe\OpenPet-<username>-control-v1`).
  - Versioned handshake and length-prefixed frame bounds (max 16 MB).
- **Residual Risk**: High-integrity process on same local machine under same user context.
- **Automated Verification**: `openpet_ipc::tests::test_frame_codec_roundtrip`.

### 2.5 Screen Privacy Boundary (`openpet-screen`)
- **Asset**: User desktop contents, private documents, passwords on screen.
- **Threat**: Eavesdropping, screen exfiltration, unauthorized OCR logging.
- **Attack Vector**: Background capture running continuously and uploading screenshots to remote providers.
- **Mitigation**:
  - Screen analysis defaults strictly to `OFF`.
  - Capture API initialization count is 0 when disabled.
  - Raw pixel buffers never leave the `openpet-screen` crate.
  - Only abstract derived categorizations (`ScreenContext`) are output.
  - Instant Privacy Mode toggle halts capture immediately.
- **Residual Risk**: Bug in process classification logic.
- **Automated Verification**: `openpet_screen::tests::test_zero_capture_initialization_when_disabled`.

### 2.6 Credential & Secret Storage (`openpet-secrets`)
- **Asset**: AI provider API keys, encryption passphrases.
- **Threat**: Token theft from configuration files, memory dumps, or log files.
- **Attack Vector**: Malware scraping plaintext `.env` files or reading crash logs.
- **Mitigation**:
  - Tokens stored in Windows Credential Manager.
  - In-memory `SecretString` implements `ZeroizeOnDrop`.
  - `Display` and `Debug` implementations strictly output `[REDACTED_SECRET]`.
- **Residual Risk**: Admin-level process accessing Credential Manager.
- **Automated Verification**: `openpet_secrets::tests::test_secret_string_formatting`.

### 2.7 Update Pipeline & Binary Distribution
- **Asset**: Application executable binaries.
- **Threat**: Man-in-the-middle binary tampering, rogue update mirrors.
- **Attack Vector**: DNS spoofing pointing update client to trojanized installer.
- **Mitigation**:
  - Cryptographically signed JSON update manifests.
  - Mandatory SHA-256 binary checksum validation prior to execution.
  - User confirmation required before launching installer.
- **Residual Risk**: Compromised developer private signing key.
