# OpenPet Architecture Overview

## 1. System Topology

OpenPet is an open-source, local-first desktop pet companion platform for Windows 11.

```text
+-------------------------------------------------------------+
|                     Windows Desktop                         |
|                                                             |
|   [ Transparent Pet Window ]          [ System Tray ]       |
|             |                                |              |
|             v                                v              |
|   +-----------------------------------------------------+   |
|   |                 openpet-host.exe                    |   |
|   |                                                     |   |
|   |   +-------------------+    +--------------------+   |   |
|   |   |   Behavior Engine |    |  Frame Scheduler   |   |   |
|   |   |   (Utility AI)    |    |  (Dynamic 5-60 FPS)|   |   |
|   |   +-------------------+    +--------------------+   |   |
|   |             |                        |              |   |
|   |   +-------------------+    +--------------------+   |   |
|   |   | Database Owner    |    | Reminder Scheduler |   |   |
|   |   | (SQLite / Storage)|    | & Notification Hub |   |   |
|   |   +-------------------+    +--------------------+   |   |
|   |             ^                                       |   |
|   |             | (Single Writer)                       |   |
|   |             v                                       |   |
|   |   +---------------------------------------------+   |   |
|   |   | Named Pipe IPC Server: \\.\pipe\OpenPet-..  |   |   |
|   |   +---------------------------------------------+   |   |
|   +-----------------------------------------------------+   |
|                             ^                               |
|                             | Named Pipe IPC                |
|                             v                               |
|   +-----------------------------------------------------+   |
|   |                openpet-control.exe                  |   |
|   |   (Iced / Dashboard Management Interface)           |   |
|   |   * Onboarding   * Pet Selector  * Chat UI          |   |
|   |   * Reminders    * Memory Facts  * Settings         |   |
|   +-----------------------------------------------------+   |
+-------------------------------------------------------------+
```

## 2. Core Architectural Guarantees

1. **Independent Process Lifecycles**: Closing the Control Center interface (`openpet-control.exe`) does not close the host daemon (`openpet-host.exe`). The pet continues animating, reacting, and managing scheduled reminders.
2. **Single Database Owner**: Only `openpet-host.exe` connects to `openpet.db`. Control Center performs all queries and mutations via Named Pipe IPC, preventing SQLite concurrency deadlocks.
3. **Deterministic Behavior Core**: Real-time locomotion and emotions run on a dedicated 10–20 Hz tick decoupled from generative AI models. If external AI services are offline or slow, the pet continues to wander, play, and sleep smoothly.
4. **Privacy-by-Default Boundary**: Screen context observation is disabled by default. When disabled, screen capture APIs are never initialized. Raw pixels never leave the `openpet-screen` crate.
5. **Sandboxed Wasm Plugins**: Community extensions execute inside an isolated WebAssembly sandbox with zero capabilities by default. Fuel quotas prevent runaway loops from freezing the host.
