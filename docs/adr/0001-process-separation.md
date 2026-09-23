# ADR 0001: Process Separation Between Host and Control Center

## Status
Accepted

## Context
A desktop pet application requires real-time desktop window rendering, 10–20 Hz deterministic behavior ticks, mouse gesture handling, and system tray integration. At the same time, users need a rich management interface (Control Center) to configure settings, review structured memory facts, manage reminders, inspect plugins, and chat with optional AI backends.

Running both user interface management and real-time desktop simulation in a single monolithic process causes severe drawbacks:
1. Closing the settings window would terminate the pet or require hiding complex UI trees in memory.
2. Heavy UI layout or garbage collection pauses would stutter the pet's desktop animation loop.
3. Database locking conflicts could occur if multiple interfaces accessed SQLite concurrently without strict ownership.

## Decision
We decouple OpenPet into two independent processes communicating via Windows Named Pipes (`\\.\pipe\OpenPet-<user>-control-v1`):
1. `openpet-host.exe`: The persistent background daemon owning the desktop transparent window, behavior engine, database writes, reminder schedules, and IPC server.
2. `openpet-control.exe`: The on-demand management interface. Closing Control Center with the 'X' button simply closes the UI window; the pet continues running undisturbed on the host.

## Consequences
### Positive
- High resilience: Control Center crashes or closing never terminates the pet.
- Clean database ownership: Only `openpet-host` opens the SQLite database.
- Low resource usage: Control Center is loaded into RAM only when the user opens it.

### Negative
- IPC overhead for sending settings updates, queries, and conversational streaming chunks.
- Requires robust framing, message validation, and protocol versioning (`ClientHello` / `ServerHello`).
