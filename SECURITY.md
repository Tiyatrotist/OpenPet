# Security Policy

## 1. Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |
| < 0.1.0 | No        |

---

## 2. Reporting a Vulnerability

**Do not report security vulnerabilities through public GitHub issues.**

If you discover a security flaw, vulnerability, or potential exploit in OpenPet, please report it privately:
- Via GitHub Security Advisory: Navigate to **Security > Advisories > Report a vulnerability**.
- Or via email to the maintainer: `179411334+Tiyatrotist@users.noreply.github.com` with the subject `[SECURITY VULNERABILITY] OpenPet`.

Please include:
1. Detailed description of the vulnerability.
2. Steps to reproduce or proof-of-concept payload (e.g. malformed PetPack or IPC message).
3. The affected crates and version.
4. Your assessment of potential impact.

We will acknowledge receipt within 48 hours and coordinate a coordinated disclosure timeline.

---

## 3. Security Design Principles

- **Local-First & Offline Capable**: OpenPet functions fully without internet access or accounts.
- **Fail-Closed Privacy**: Local screen observation defaults strictly to `OFF`. Capture APIs are never initialized when disabled.
- **Least Privilege Sandboxing**: Plugins execute in WebAssembly with zero initial capabilities.
- **Zero Secret Exposure**: Tokens are isolated inside Windows Credential Manager and memory buffers are zeroized upon drop.
- **Confirmed Tool Invocations**: Generative AI models cannot perform mutations (e.g. creating/deleting reminders) without explicit user confirmation.
