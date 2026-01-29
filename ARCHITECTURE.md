# 🏛️ Architecture: Silent-Rails Engine

This document details the technical implementation of the **North Architecture** "Silent-Rails" protocol.

### ⚡ The Decoupled Execution Model
Our architecture achieves **66ms latency** by separating the validation layer from the execution rail:
1.  **The Sentinel Program (Rust/Anchor):** Manages state transitions and cryptographic "Seals".
2.  **Privacy Rail Layer:** A fragmented account structure that prevents global state sniffing by third-party trackers.

### 🛡️ Helius Infrastructure Integration
Silent-Rails is optimized for **Helius RPCs**:
* **Transaction Indexing:** We use Helius DAS (Digital Asset Standard) to maintain sub-100ms response times for institutional wallets.
* **Compute Unit (CU) Efficiency:** Our custom obfuscation logic is designed to use 15% fewer CUs than standard anonymous transfers, ensuring scalability.

### 🌑 Data Fragmentation
To ensure "Silence", transaction data is not stored in a single account. We distribute state across multiple cryptographic nodes, making the transaction graph invisible to anyone without the original decryption key.
