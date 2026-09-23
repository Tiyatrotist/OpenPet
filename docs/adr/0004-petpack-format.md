# ADR 0004: PetPack V1 Archive Specification & Security Sandbox

## Status
Accepted

## Context
Community extensibility is a core principle of OpenPet. Users and creators need to share custom pets containing sprite atlases, behavior configurations, translations, and sound effects.

Allowing arbitrary files or untrusted archives introduces attack vectors including directory traversal (Zip-Slip), decompression bombs (Zip Bombs), denial-of-service via huge allocations, and overwrite attacks on system files.

## Decision
We define the `.openpet` container format based on ZIP with strict validation constraints:
1. Container Constraints:
   - Archive size <= 256 MB.
   - Total uncompressed size <= 512 MB.
   - Entry count <= 2,000 files.
   - Single asset size <= 128 MB.
2. Path Safety:
   - All entry paths must be strictly relative normal components (`Component::Normal`).
   - Absolute paths, parent traversal (`..`), symlinks, hardlinks, and NTFS alternate data streams (`:`) are rejected immediately.
3. Cryptographic Verification:
   - All included assets are hashed using SHA-256 and matched against manifest entries.
4. Atomic Installation:
   - Unpacking occurs strictly into an isolated staging directory (`staging/stage_<uuid>`).
   - The verified directory is atomically moved to `packs/<pet_id>`. Broken or malicious packages never corrupt live state.

## Consequences
### Positive
- Robust defense against malicious community packages.
- Easy distribution of self-contained `.openpet` files.
- Command-line tooling (`openpet-pack`) enables creators to validate and build packages.

### Negative
- CPU overhead for computing SHA-256 digests during installation.
