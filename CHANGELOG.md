# Changelog

All notable changes to OpenPet will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- **Dual Art-Style Companion Engine**:
  - **Photorealistic Companion (Gerçek Kedi)**: Added Oreo the Tuxedo Cat (`oreo-tuxedo-cat`), featuring high-resolution photographic cutout frames (`sit`, `idle`, `high_five`, `sleep`, `stretch`, `curious`, `ask`) derived from real companion photos with flood-fill background removal and smooth alpha scaling.
  - **Procedural Pixel Art Companion**: Enriched Mimi with 7 customizable breeds and coat patterns (Tabby, Tuxedo, Calico, Ginger, Siamese, Black, White).
  - **Art Style Switcher**: Instant switching between Pixel Art and Realistic styles directly from the Control Center status card, application settings, desktop pet right-click context menu, and system tray.
  - **Official Package**: Added `packs/official/realistic-cat` with official V1 schema manifest and localization.

## [0.1.0-alpha.1] - 2026-09-23

### Highlights
- Initial bootstrap of the OpenPet desktop platform for Windows 11.
- Complete modular Rust workspace comprising 18 core domain crates and 3 target applications.
- Deterministic 10–20 Hz Utility AI behavior engine supporting 16+ semantic states.
- Local-first architecture requiring zero network connection or account sign-up.

### Added
- **Core Applications**:
  - `openpet-host.exe`: Background daemon owning pet lifecycle, database, behavior tick, and IPC server.
  - `openpet-control.exe`: Management interface for settings, pet switching, memory, reminders, and chat.
  - `openpet-pack.exe`: Creator CLI for building, inspecting, hashing, validating, and signing `.openpet` packages.
- **Security & Privacy Boundaries**:
  - `openpet-screen`: Strict zero-capture guarantee when disabled; instant Privacy Mode toggle.
  - `openpet-secrets`: Windows Credential Manager integration with memory zeroization on drop.
  - `openpet-petpack`: Zip-Slip protection, decompression bomb prevention, and atomic staging installation.
  - `openpet-plugin-host`: Sandboxed WebAssembly execution with fuel limits and capability isolation.
  - `openpet-reminders`: Mandatory human-in-the-loop confirmation gate for LLM tool mutations.
- **Official Assets**:
  - `mimi-cat`: Official starter companion cat with sprite atlas, behavior maps, and English/Turkish translations.
- **Documentation**:
  - Complete Architecture Decision Records (`0001` through `0008`) under `docs/adr/`.
  - Comprehensive threat model under `docs/security/threat-model.md`.
