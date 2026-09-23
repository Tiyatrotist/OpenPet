# ADR 0002: Rendering Stack & Transparent Desktop Pet Lifecycle

## Status
Accepted

## Context
Desktop companion applications must render transparent, borderless sprite frames directly on top of arbitrary Windows 11 desktop environments without blocking clicks meant for underlying windows.

Historically, desktop pets relied on Win32 GDI layered windows (`UpdateLayeredWindow`), legacy DirectDraw, or heavyweight web wrappers (Electron), which consume high CPU/RAM and suffer from visual artifacts or high latency.

## Decision
We select `winit` + `wgpu` targeting modern DirectX 12 on Windows 11, coupled with `windows-sys` for platform-specific window style configuration:
1. `WS_EX_LAYERED` and `WS_EX_NOREDIRECTIONBITMAP` for alpha blending and composition.
2. Per-Monitor DPI Awareness V2 (`SetProcessDpiAwarenessContext`) to ensure crisp sprite scaling between 100% and 250% DPI across multiple monitors.
3. Compact 1-bit per pixel precomputed Hit Masks (`HitMask`) to evaluate click transparency instantly without reading back GPU textures.
4. Dynamic frame scheduling: 60 FPS during locomotion, 24 FPS during idle states, and 5 FPS during sleep states.

## Consequences
### Positive
- Sub-50ms interaction latency.
- Very low idle CPU (<1% average) and low baseline RAM (<150 MB).
- Native DirectX 12 composition without web engine bloat.

### Negative
- Requires robust device loss recovery and monitor hotplug handling.
