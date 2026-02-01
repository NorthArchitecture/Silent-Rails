![Security](https://img.shields.io/badge/Security-Hardened-brightgreen?logo=shield)
![Audit](https://img.shields.io/badge/Audit-Ready-blueviolet)
![Status](https://img.shields.io/badge/Phase-Architectural--V2.0-blue)

# 🛡️ Security & Privacy Standards

Silent-Rails is built on a "Security-First" architecture, ensuring that privacy never compromises performance. Our core engine has been stress-tested to maintain a stable **66.00ms latency** under heavy institutional load.

### 🔒 Privacy Integrity
* **Zero-Knowledge Evidence**: We utilize 32-byte ZK-evidences to ensure sensitive data is never stored in plain-text on the Solana ledger.
* **Deterministic PDA Isolation**: Handshakes are secured via unique Program Derived Addresses using `[authority, fragment_id]` as seeds, preventing cross-account data leakage and replay attacks.
* **Programmable Audit Seals**: Native support for encrypted audit trails allows for regulatory compliance without sacrificing user anonymity.
* **Mathematical Hardening**: All state transitions are protected by native **overflow-checks** and Anchor’s `has_one = authority` security constraints.

### ⚡ Performance Validation
* **High-Capacity Stress Test**: Our handshake protocols are verified against **25,000,000 iterations** of intensive cryptographic work.
* **Decoupled Architecture**: By separating Privacy Seals from the execution layer, we eliminate Sealevel runtime bottlenecks, supporting over **185k TX/Sample**.

### 🕵️ Auditability & Compliance
While transactions are private, they remain **auditable** for the entities involved. The `sentinel` program generates a unique **Audit Seal** for every rail movement, ensuring that institutional compliance requirements are met without public exposure.

### 📞 Reporting a Vulnerability
If you discover a security vulnerability, please contact us immediately for a coordinated disclosure.
* **Primary Contact**: Direct Message on **X @North_Protocol**
* **Response SLA**: We aim to acknowledge all critical security reports within 24 hours.
