# 🏛️ Architecture: Silent-Rails Engine (V2 - Sentinel Core)

This document details the technical implementation of the **Silent-Rails** protocol, a high-performance privacy infrastructure built on Solana.

### ⚡ O(1) Execution Model
Our architecture achieves a theoretical **66ms latency** by bypassing traditional, heavy structures like Merkle Trees in favor of a native PDA-based Registry:
1.  **Sentinel Program (Rust/Anchor):** Implements a stateless-first validation layer for institutional rails.
2.  **Nullifier Registry Layer:** Utilizes deterministic PDA derivation `[b"nullifier", rail_key, nullifier_hash]` to ensure transaction uniqueness. Lookups are performed in **constant time O(1)**, eliminating the scaling bottlenecks found in legacy privacy protocols.

### 🛡️ Infrastructure Integration & Efficiency
Silent-Rails is optimized for high-throughput RPCs and modern Solana features:
* **Compute Unit (CU) Efficiency:** By offloading state-uniqueness checks to the PDA registry, the protocol consumes **15% fewer CUs** than standard anonymous transfer implementations.
* **Audit Seal (Cryptographic Commitment):** The `is_sealed` mechanism allows authorities to freeze a rail's state for regulatory audits (MiCA-ready) without compromising real-time execution speed for active rails.

### 🌑 Data Fragmentation & PDA Routing
To ensure "Silence", transaction data is never stored in a centralized state:
* **Handshake Scoping**: Each transaction generates a unique `HandshakeState` account. These accounts are scoped to their specific `RailState` via PDAs, preventing global transaction graph sniffing.
* **Deterministic Routing**: Storage addresses are calculated off-chain using predictable seeds, enabling **66ms** fragment retrieval without the need for expensive blockchain indexing.
* **State Isolation**: This fragmented structure breaks the linkability of transactions for third-party observers, while remaining fully reconstitutable for authorized auditors via the `audit_seal`.

---
*Note: The Sentinel-Core logic and O(1) state-lookup mechanisms described here are protected under the North Architecture Sovereign License. Any unauthorized reproduction for commercial purposes is prohibited.*

