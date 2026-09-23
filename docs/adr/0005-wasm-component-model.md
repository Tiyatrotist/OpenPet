# ADR 0005: Sandboxed WebAssembly & Capability-Based Plugin Architecture

## Status
Accepted

## Context
Desktop extensibility must never compromise user security or desktop stability. Traditional plugin architectures relying on native dynamic libraries (C/C++ `.dll`) permit plugins to access arbitrary memory, execute arbitrary system processes, steal tokens, or crash the entire host process via segmentation faults or unhandled panics.

## Decision
1. Native DLL plugins are strictly forbidden in OpenPet V1.
2. All third-party plugins execute inside a sandboxed WebAssembly (Wasm) runtime with a stable WIT / ABI contract (`openpet-plugin-api`).
3. Principle of Least Privilege: Plugins have zero capabilities by default (`NONE`). Capabilities (`pet.read_state`, `pet.command`, `notifications`, `reminders.read`, `reminders.write`, `memory.read`) must be explicitly declared and granted.
4. Host Protection:
   - Plugins have hard memory budgets and fuel limits (`DEFAULT_PLUGIN_FUEL_BUDGET`).
   - If a plugin infinite-loops, runs out of fuel, or panics, the host catches the error, marks the plugin as faulted, disables it, and notifies the user without hanging or terminating the host.
5. Plugins are denied raw filesystem, raw network, raw screen buffers, process execution, and Windows APIs.

## Consequences
### Positive
- Immune to crashes caused by third-party plugins.
- Guaranteed safety against malware or unverified community mods.

### Negative
- Sandboxing overhead compared to native code execution.
