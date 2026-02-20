# 🏛️ Architecture: Silent-Rails Engine (V2 - Sentinel Core)

This document details the technical implementation of the **Silent-Rails** protocol, a high-performance zero-knowledge privacy infrastructure built on Solana.

---

## ⚡ O(1) Execution Model
Our architecture achieves **constant-time** privacy operations by bypassing traditional Merkle Trees in favor of a native PDA-based Registry:
1.  **Sentinel Program (Rust/Anchor):** Implements a stateless-first validation layer with on-chain Groth16 proof verification for institutional privacy rails.
2.  **Nullifier Registry Layer:** Utilizes deterministic PDA derivation `[b"nullifier", rail_key, nullifier_hash]` to ensure transaction uniqueness. Lookups are performed in **constant time O(1)**, eliminating the scaling bottlenecks found in legacy privacy protocols.
3.  **Memory Optimization Layer:** Critical token transfer/deposit account contexts are implemented with Boxed Accounts to reduce stack footprint and preserve low-latency execution.

---

## 🔐 ZK Privacy Stack

### Groth16 On-Chain Verifier
The Sentinel program includes a native Groth16 verifier operating over the **alt_bn128** elliptic curve via Solana precompiles:
- **Proof size:** 256 bytes fixed (A: G1, B: G2, C: G1)
- **Pairing check:** 4-element pairing via `alt_bn128_pairing` syscall (768 bytes input)
- **IC accumulation:** Linear combination of public inputs with `alt_bn128_multiplication` and `alt_bn128_addition`
- **Verification keys:** Real parameters from trusted setup ceremony (Powers of Tau pot16 + phase 2 contributions)

### ZK Circuits (circom)
Three circuits enforce privacy constraints off-chain, with proofs verified on-chain:

| Circuit | Public Inputs | Purpose |
| :--- | :--- | :--- |
| `sentinel_commitment` | commitment, nullifier_hash | Proves knowledge of secret behind a Poseidon commitment |
| `sentinel_transfer` | sender_before, sender_after, receiver_before, receiver_after, nullifier | Proves valid balance transfer without revealing amounts |
| `sentinel_withdraw` | balance_before, balance_after, amount, nullifier | Proves sufficient balance for withdrawal |

### Cryptographic Primitives
- **Poseidon Hash:** `commitment = Poseidon(secret, amount)` — algebraically efficient, SNARK-friendly
- **ElGamal Encryption:** Homomorphic encrypted balances (64 bytes) for confidential balance tracking
- **Scoped Nullifiers:** `nullifier_hash = Poseidon(secret)` — prevents double-spend without revealing identity

---

## 🏗️ Account Architecture (PDA Layout)

```
RailState           [b"rail", authority]
├── ZkVault         [b"zk_vault", rail]
├── VaultAssetState [b"asset_vault", rail, asset_key]
├── HandshakeState  [b"handshake", rail, nullifier_hash]
├── NullifierRegistry [b"nullifier", rail, nullifier_hash]
├── DepositRecord   [b"deposit", rail, sender, counter]
├── TokenDepositRecord  [b"token_deposit", rail, sender, mint, counter]
├── TransferRecord  [b"transfer", sender_rail, receiver_rail, transfer_nonce]
└── VaultPool       [b"vault_pool", rail]
```

Each account is deterministically derived, enabling **O(1) lookups** without on-chain indexing.

---

## 🔄 Transaction Flows

### Deposit (SOL or Token)
```
Client: generate secret → compute commitment = Poseidon(secret, amount)
Client: generate Groth16 proof (commitment circuit)
On-chain: verify proof → transfer SOL/tokens to vault → store encrypted balance
Client: generate secret → compute commitment = Poseidon(secret, amount)
Client: generate Groth16 proof (commitment circuit)
On-chain: verify proof → increment deposit counter → derive PDA with counter → transfer SOL/tokens to vault → store encrypted balance
```

### Confidential Transfer
```
Client: compute new commitments for sender & receiver
Client: generate Groth16 proof (transfer circuit)
On-chain: verify proof → verify commitment matches → update both vaults
```

### Withdraw
```
Client: compute new balance commitment after withdrawal
Client: generate Groth16 proof (withdraw circuit)
On-chain: verify proof → verify balance sufficient → transfer from vault → update state
```

---

## 🛡️ Security Model

### On-Chain Enforcement
- **Groth16 verification** on every deposit, transfer, and withdrawal — no state change without valid proof
- **Commitment checks** — on-chain balance commitment must match before any operation
- **Nullifier uniqueness** — PDA-based, impossible to reuse without program modification
- **Transfer nonce replay resistance** — transfer PDAs are nonce-bound to sender/receiver rails, preventing replay of confidential transfer records
- **Authority isolation** — `has_one = authority`, PDA seed constraints, and receiver authority checks enforce strict signer ownership boundaries
- **Multi-asset state isolation** — each rail/asset pair uses an independent VaultAssetState PDA for SOL and SPL token separation

### Rail Lifecycle
```
Active → Paused → Active (unpause)
Active → Sealed (immutable, audit-ready)
Active → Deactivated (with reason code)
```

### Infrastructure Efficiency
- **Compute Unit (CU) Efficiency:** PDA-based registry consumes fewer CUs than Merkle Tree alternatives
- **Fixed proof size:** 256 bytes regardless of transaction complexity
- **No global state:** Each rail is fully isolated — no cross-rail data leakage

---

## 🌑 Data Fragmentation & Privacy Guarantees

To ensure "Silence", transaction data is never stored in a centralized state:
- **Handshake Scoping**: Each transaction generates a unique `HandshakeState` account scoped to its specific `RailState` via PDAs, preventing global transaction graph analysis.
- **Encrypted Balances**: All balance data is ElGamal-encrypted. Only the vault owner can decrypt.
- **Zero On-Chain Amounts**: Confidential transfers expose zero plaintext amounts. Only ZK proofs and encrypted ciphertext are stored.
- **State Isolation**: Fragmented structure breaks transaction linkability for third-party observers, while remaining fully reconstitutable for authorized auditors via the `audit_seal`.

---

## ✅ Critical Reliability Validation

The current institutional validation suite focuses on production-critical paths and security invariants:
- **26 critical tests passed**
- **Success-path coverage:** deposit, confidential transfer, withdrawal
- **Multi-asset coverage:** SOL/SPL Asset State isolation and token path constraints
- **Security coverage:** nullifier anti-replay, transfer nonce integrity, PDA authority isolation

---

## 📋 Trusted Setup

The Groth16 verification keys were generated through a formal ceremony:
1. **Powers of Tau** (pot16, 2^16 constraints) — universal phase 1
2. **Phase 2 contributions** — per-circuit randomness injection
3. **Verification key extraction** — embedded directly in `lib.rs` as static byte arrays

Circuits can be recompiled and the ceremony re-executed via `setup.sh`.

---
*The Sentinel-Core logic, Groth16 verification circuits, and O(1) state-lookup mechanisms described here are protected under the North Architecture Sovereign License. Any unauthorized reproduction for commercial purposes is prohibited.*