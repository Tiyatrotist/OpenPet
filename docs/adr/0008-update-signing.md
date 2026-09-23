# ADR 0008: Signed Update Manifests & Supply-Chain Integrity

## Status
Accepted

## Context
Automatic updates for desktop Windows executables running with user privileges represent an attractive target for man-in-the-middle (MITM) attacks and DNS poisoning. Relying strictly on transport-layer HTTPS does not protect users against compromised CDNs, rogue proxies, or tampered download mirrors.

## Decision
1. Application updates are governed by signed JSON update manifests containing:
   - Target SemVer version.
   - Release channel (`stable`, `beta`).
   - Download URL.
   - SHA-256 artifact hash.
   - File size in bytes.
   - Publisher Ed25519 signature.
2. Verification Pipeline:
   - Download manifest over HTTPS.
   - Verify publisher signature against embedded public key.
   - Compare version against current running binary.
   - Download binary artifact into isolated `updates/` staging directory.
   - Verify cryptographic SHA-256 digest against manifest.
   - Present update prompt to user before executing replacement installer.

## Consequences
### Positive
- Defense-in-depth against supply chain and mirror tampering.
- Clean separation between release channels.

### Negative
- Maintainer must manage and protect offline signing keys for all public releases.
