# Contributing to OpenPet

Thank you for your interest in contributing to OpenPet! We welcome contributions that align with our core values: local-first reliability, desktop performance, user privacy, and clean Rust architecture.

---

## 1. Development Prerequisites

- **OS**: Windows 11 x64 (Primary supported runtime).
- **Rust Toolchain**: Rust 1.80+ stable (`rustup default stable`).
- **Required Cargo Components**: `rustfmt`, `clippy`.

---

## 2. Getting Started

1. Fork and clone the repository:
   ```powershell
   git clone https://github.com/Tiyatrotist/OpenPet.git
   cd OpenPet
   ```
2. Verify local compilation and tests:
   ```powershell
   cargo check --workspace
   cargo test --workspace
   cargo clippy --workspace --all-targets
   cargo fmt --check
   ```

---

## 3. Core Architectural Rules

1. **Local-First Above All**: Core desktop pet behaviors, physics, animations, and database features must work without an active internet connection or AI API key.
2. **Never Block the Render Loop**: Network I/O, LLM streaming, disk writes, and OCR analysis must execute asynchronously off the UI thread.
3. **Strict Privacy**: Never capture screen frames or OCR text when `screen_analysis_enabled` is false. Raw pixels must never leave the `openpet-screen` crate.
4. **No Unconfirmed LLM Mutations**: If an AI model proposes modifying reminders or database records, an explicit user confirmation prompt is mandatory.
5. **No Native DLL Plugins**: Plugins must execute through the sandboxed WebAssembly runtime.

---

## 4. Submitting a Pull Request

- Keep diffs focused on a single logical change or bug fix.
- Ensure all new logic includes unit or contract tests.
- Verify `cargo fmt --check` and `cargo clippy --workspace --all-targets` pass with zero warnings before pushing.
- Respect our public identity: Use `Tiyatrotist` in documentation examples. Never commit personal paths, usernames, or private API keys.
