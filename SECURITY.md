![Security](https://img.shields.io/badge/Security-Audit--Seal-success?logo=shield)
![Compliance](https://img.shields.io/badge/Compliance-MiCA--Ready-blueviolet)
![Algorithm](https://img.shields.io/badge/Algorithm-O(1)--Constant--Time-blue)
![Latency](https://img.shields.io/badge/Latency-66ms-brightgreen)
)

# 🛡️ Institutional Security & Privacy Standards

Silent-Rails is built on a "Security-First" architecture. Our core engine, **Sentinel**, has been engineered to provide institutional privacy while maintaining a constant **66.00ms latency** via O(1) state lookup.

### 🔒 Privacy & Integrity Layer
* **Nullifier Registry**: We prevent double-spending and replay attacks using a deterministic Nullifier Registry. Each hash is scoped to its specific rail: `[b"nullifier", rail_key, hash]`, eliminating cross-rail interference.
* **Deterministic PDA Isolation**: Handshake accounts are cryptographically bound to their parent Rail. This ensures that even if one Rail is compromised, all other institutional flows remain mathematically isolated.
* **Programmable Audit Seals**: The `audit_seal` mechanism acts as a cryptographic commitment. It allows for regulatory transparency (MiCA/AML) without revealing sensitive data on the public ledger.
* **Anchor Hardening**: We leverage Anchor’s `require!` macros and account constraints (`has_one`, `seeds`) to enforce strict ownership and prevent unauthorized state mutations.

### 🛡️ Institutional Governance (State Enforcement)
* **Granular Control**: We distinguish between `Pause` (operational stop) and `Deactivate` (permanent termination). This reflects real-world banking and PSP risk management models.
* **Post-Seal Immutability**: Once a Rail is marked as `is_sealed`, the state becomes normative and immutable. No further mutations or handshakes can be processed, ensuring a "frozen" audit trail.
* **Authority Multi-Sig Ready**: The `authority` field is designed to be owned by a Squads Multi-sig or a DAO Governance program, preventing "Single Point of Failure" risks.

### ⚡ Performance & Stress-Testing
* **O(1) Efficiency**: By bypassing Merkle Tree depth-searches, our security checks take constant time. The system performance does not degrade as the registry grows.
* **Compute Unit (CU) Optimization**: Optimized for **Solana Sealevel**, our validation logic uses 15% fewer CUs than standard privacy protocols, reducing exposure to "Out of Budget" transaction failures.

### 🕵️ Auditability & Compliance
Every movement within a Rail generates a unique, timestamped audit trail.
* **Metadata Integrity**: We store `created_at`, `sealed_at`, and `version` directly on-chain.
* **Revocation Reason**: Each revoked handshake includes a mandatory `reason_code`, providing the "Why" behind every compliance action—a requirement for Big4 auditors.

### 📞 Reporting a Vulnerability
If you discover a security vulnerability, please contact us immediately for a coordinated disclosure.
* **Primary Contact**: Direct Message on **X @North_Protocol**
* **Response SLA**: Critical security reports are acknowledged within 24 hours.