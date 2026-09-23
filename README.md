# OpenPet

[![CI](https://github.com/Tiyatrotist/OpenPet/actions/workflows/ci.yml/badge.svg)](https://github.com/Tiyatrotist/OpenPet/actions/workflows/ci.yml)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Release](https://img.shields.io/badge/release-v0.1.0--alpha.1-green.svg)](CHANGELOG.md)

OpenPet is an open-source, local-first desktop pet and companion platform for Windows 11. It brings living, customizable companions to your desktop with deterministic utility AI, transparent sprite rendering, and extensible plugin architecture.

---

## What is OpenPet?
- An open-source desktop companion that lives, walks, plays, and sleeps naturally on your Windows desktop.
- A local-first application: fully usable without internet access, accounts, or AI API keys.
- An extensible platform supporting community `.openpet` packages and sandboxed WebAssembly plugins.
- A private local assistant capable of managing reminders and learning structured, user-auditable memory facts.

## What is OpenPet NOT?
- It is NOT an Electron or web-wrapper app (built purely in Rust with native Windows APIs).
- It is NOT a cloud-dependent chatbot avatar (the pet behavior engine runs independently of generative AI).
- It is NOT spyware (local screen analysis defaults to OFF, and raw screen buffers never leave your machine).

---

## Project Status & Maturity
- **Current Version**: `0.1.0-alpha.1` (Active pre-release development).
- **Supported Platform**: Windows 11 x64 (DirectX 12 / Win32).

---

## Core Capabilities
- **Deterministic Utility AI Engine**: 16+ semantic states (`idle`, `walk`, `run`, `sit`, `sleep`, `wake`, `play`, `pet`, `high_five`, etc.) evaluated at 10–20 Hz with natural circadian rhythm transitions.
- **Direct Desktop Interactions**: Drag-and-drop with position tracking, petting, high fives, and single/double-click reactions.
- **Dynamic Frame Scheduler**: Adapts frame rendering between 5 FPS (deep sleep), 24 FPS (idle), and 60 FPS (active locomotion).
- **Secure `.openpet` Bundles**: Self-contained ZIP packages validated with SHA-256 checks, Zip-Slip prevention, and atomic staging.
- **Sandboxed WebAssembly Plugins**: Safe third-party extensibility powered by capability-based permissions (`NONE` by default).
- **Local SQLite Storage**: Encrypted at-rest context with automated transactional schema migrations and FTS search.
- **Privacy by Architecture**: Screen analysis strictly defaults to OFF. Instant Privacy Mode halts capture immediately.
- **Optional AI Companion Layer**: Normalized multi-provider streaming (Ollama, OpenAI, Anthropic, Gemini) with human confirmation required for all tool mutations.

---

## Architecture

```text
+-------------------------------------------------------------+
|                      Windows Desktop                        |
|                                                             |
|   [ Transparent Pet Window ]          [ System Tray ]       |
|             |                                |              |
|             v                                v              |
|   +-----------------------------------------------------+   |
|   |                 openpet-host.exe                    |   |
|   |   - Behavior Engine (Utility AI, 10-20 Hz)          |   |
|   |   - Database Owner (SQLite / Storage Layer)         |   |
|   |   - Frame Scheduler (Dynamic 5-60 FPS)              |   |
|   |   - Reminder Scheduler & Notification Dispatcher    |   |
|   |   - Named Pipe IPC Server: \\.\pipe\OpenPet-...     |   |
|   +-----------------------------------------------------+   |
|                             ^                               |
|                             | Named Pipe IPC                |
|                             v                               |
|   +-----------------------------------------------------+   |
|   |                openpet-control.exe                  |   |
|   |   - Management & Control Plane                      |   |
|   |   - Onboarding, Pet Selector, Chat, Reminders,      |   |
|   |     Memory, Plugins, Privacy, Settings              |   |
|   +-----------------------------------------------------+   |
+-------------------------------------------------------------+
```

For detailed architectural decisions, see the [Architecture Decision Records (ADRs)](docs/adr/).

---

## Quick Start

### Prerequisites
- Windows 11 x64
- Rust 1.80+ (`rustup default stable`)

### Building from Source
```powershell
# Clone the repository
git clone https://github.com/Tiyatrotist/OpenPet.git
cd OpenPet

# Build all workspace targets
cargo build --workspace

# Run the test suite
cargo test --workspace
```

### Running OpenPet
```powershell
# Start the background host daemon
cargo run -p openpet-host

# In a separate terminal, launch the Control Center
cargo run -p openpet-control
```

### Managing Pet Packages
```powershell
# Validate the included official pet
cargo run -p openpet-pack -- validate packs/official/mimi-cat

# Inspect package contents
cargo run -p openpet-pack -- inspect packs/official/mimi-cat.openpet
```

---

## Configuration & Privacy

Configuration is managed via `%LOCALAPPDATA%\OpenPet\data\openpet.db`.

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `screen_analysis_enabled` | `false` | Local desktop activity analysis (strictly opt-in) |
| `privacy_mode` | `false` | When true, immediately suspends capture and purges temporary buffers |
| `always_on_top` | `true` | Keeps pet window above standard desktop windows |
| `animation_quality` | `High` | Frame rate target mode (Low: 30 FPS, High: 60 FPS) |

---

## Known Limitations in V1
- Windows 11 x64 is the sole supported platform (macOS, Linux, and mobile are roadmap items).
- Generative pet creation from photos requires an external AI provider key or local ComfyUI instance.
- Screen context awareness produces categorical activity signals (`Meeting`, `Video`, `Writing`); full OCR text is intentionally not extracted to protect privacy.

---

## Contributing
We welcome contributions! Please review [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md) before opening pull requests or reporting security issues.

---

## License
OpenPet is licensed under the [GNU Affero General Public License v3.0 (AGPL-3.0)](LICENSE).
