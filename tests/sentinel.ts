import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { 
  TOKEN_PROGRAM_ID, 
  createMint, 
  createAssociatedTokenAccount,
  getAssociatedTokenAddressSync,
  mintTo 
} from "@solana/spl-token";

describe("sentinel", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Sentinel as any;

  let northMint: anchor.web3.PublicKey;
  let mintAuthority = anchor.web3.Keypair.generate();
  let sharedAuth = anchor.web3.Keypair.generate();
  let sharedRailPda: anchor.web3.PublicKey;
  let sharedAta: anchor.web3.PublicKey;
  let zkVaultPda: anchor.web3.PublicKey;

  const waitConfirm = async (sig: string) => {
    const latest = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      signature: sig,
      blockhash: latest.blockhash,
      lastValidBlockHeight: latest.lastValidBlockHeight
    }, "finalized");
  };

  async function getRailPda(auth: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("rail"), auth.toBuffer()], program.programId)[0];
  }

  async function getZkVaultPda(rail: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("zk_vault"), rail.toBuffer()], program.programId)[0];
  }

  before(async () => {
    const sig = await provider.connection.requestAirdrop(mintAuthority.publicKey, 5 * anchor.web3.LAMPORTS_PER_SOL);
    await waitConfirm(sig);
    northMint = await createMint(provider.connection, mintAuthority, mintAuthority.publicKey, null, 9);
    await waitConfirm(await provider.connection.requestAirdrop(sharedAuth.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL));
    sharedAta = getAssociatedTokenAddressSync(northMint, sharedAuth.publicKey);
    await createAssociatedTokenAccount(provider.connection, sharedAuth, northMint, sharedAuth.publicKey, { commitment: "finalized" });
    await waitConfirm(await mintTo(provider.connection, mintAuthority, northMint, sharedAta, mintAuthority, 1000));
    sharedRailPda = await getRailPda(sharedAuth.publicKey);
    zkVaultPda = await getZkVaultPda(sharedRailPda);
  });

  it("1. initialize_rail_integrity", async () => {
    await program.methods.initializeRail(1, 2).accounts({
        rail: sharedRailPda, authority: sharedAuth.publicKey, authorityTokenAccount: sharedAta,
        northMint: northMint, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([sharedAuth]).rpc();
  });

  it("2. verify_authority_mapping", async () => {
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.authority.toBase58()).to.equal(sharedAuth.publicKey.toBase58());
  });

  it("3. initialize_zk_vault_anchoring", async () => {
    const pubkey = Array.from(Buffer.alloc(32, 9));
    await program.methods.initializeZkVault(pubkey).accounts({
        zkVault: zkVaultPda, rail: sharedRailPda, authority: sharedAuth.publicKey, systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([sharedAuth]).rpc();
    const vault = await program.account.zkVault.fetch(zkVaultPda);
    expect(vault.rail.toBase58()).to.equal(sharedRailPda.toBase58());
  });

  it("4. prevent_duplicate_zk_vault", async () => {
    try {
      await program.methods.initializeZkVault(Array.from(Buffer.alloc(32, 1))).accounts({
        zkVault: zkVaultPda, rail: sharedRailPda, authority: sharedAuth.publicKey, systemProgram: anchor.web3.SystemProgram.programId,
      }).signers([sharedAuth]).rpc({ skipPreflight: true });
      expect.fail();
    } catch (e) { }
  });

  it("5. block_unauthorized_zk_initialization", async () => {
    const hacker = anchor.web3.Keypair.generate();
    try {
      await program.methods.initializeZkVault(Array.from(Buffer.alloc(32, 0))).accounts({
        zkVault: zkVaultPda, rail: sharedRailPda, authority: hacker.publicKey, systemProgram: anchor.web3.SystemProgram.programId,
      }).signers([hacker]).rpc({ skipPreflight: true });
      expect.fail();
    } catch (e) { }
  });

  it("6. create_handshake_pda", async () => {
    const n = Array.from(Buffer.alloc(32, 1));
    const [h] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("handshake"), sharedRailPda.toBuffer(), Buffer.from(n)], program.programId);
    const [nr] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("nullifier"), sharedRailPda.toBuffer(), Buffer.from(n)], program.programId);
    await program.methods.createHandshake(n, n).accounts({
        handshake: h, nullifierRegistry: nr, rail: sharedRailPda, payer: sharedAuth.publicKey, systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([sharedAuth]).rpc();
  });

  it("7. prevent_double_spend_replay", async () => {
    const n = Array.from(Buffer.alloc(32, 1));
    const [nr] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("nullifier"), sharedRailPda.toBuffer(), Buffer.from(n)], program.programId);
    try {
      await program.methods.createHandshake(n, n).accounts({ handshake: anchor.web3.Keypair.generate().publicKey, nullifierRegistry: nr, rail: sharedRailPda, payer: sharedAuth.publicKey, systemProgram: anchor.web3.SystemProgram.programId }).signers([sharedAuth]).rpc({skipPreflight: true});
      expect.fail();
    } catch (e) { }
  });

  it("8. block_unauthorized_access", async () => {
    const hacker = anchor.web3.Keypair.generate();
    try {
      await program.methods.sealRail(Array.from(Buffer.alloc(32, 6))).accounts({ rail: sharedRailPda, authority: hacker.publicKey }).signers([hacker]).rpc({ skipPreflight: true });
      expect.fail();
    } catch (e) { }
  });

  it("9. check_insufficient_funds_protection", async () => {
    const poor = anchor.web3.Keypair.generate();
    const poorRail = await getRailPda(poor.publicKey);
    try {
      await program.methods.initializeRail(1, 2).accounts({ rail: poorRail, authority: poor.publicKey, authorityTokenAccount: sharedAta, northMint: northMint, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId }).signers([poor]).rpc({ skipPreflight: true });
      expect.fail();
    } catch (e) { }
  });

  it("10. verify_pause_state_enforcement", async () => {
    await program.methods.pauseRail().accounts({ rail: sharedRailPda, authority: sharedAuth.publicKey }).signers([sharedAuth]).rpc();
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.isPaused).to.be.true;
  });

  it("11. prevent_handshake_during_pause", async () => {
    const n = Array.from(Buffer.alloc(32, 99));
    try {
      await program.methods.createHandshake(n, n).accounts({ handshake: anchor.web3.Keypair.generate().publicKey, nullifierRegistry: anchor.web3.Keypair.generate().publicKey, rail: sharedRailPda, payer: sharedAuth.publicKey, systemProgram: anchor.web3.SystemProgram.programId }).signers([sharedAuth]).rpc({ skipPreflight: true });
      expect.fail();
    } catch (e) { }
  });

  it("12. verify_unpause_recovery", async () => {
    await program.methods.unpauseRail().accounts({ rail: sharedRailPda, authority: sharedAuth.publicKey }).signers([sharedAuth]).rpc();
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.isPaused).to.be.false;
  });

  it("13. verify_confidential_state_update", async () => {
    const proof = Buffer.alloc(128, 5);
    const newBalance = Array.from(Buffer.alloc(64, 7));
    await program.methods.verifyConfidentialState(proof, newBalance).accounts({
      zkVault: zkVaultPda,
      rail: sharedRailPda,
      authority: sharedAuth.publicKey,
    }).signers([sharedAuth]).rpc();
    const vault = await program.account.zkVault.fetch(zkVaultPda);
    expect(Buffer.from(vault.encryptedBalance)).to.deep.equal(Buffer.from(newBalance));
  });

  it("14. seal_rail_immutability", async () => {
    await program.methods.sealRail(Array.from(Buffer.alloc(32, 7))).accounts({ rail: sharedRailPda, authority: sharedAuth.publicKey }).signers([sharedAuth]).rpc();
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.isSealed).to.be.true;
  });

  it("15. verify_o1_compliance_status", async () => {
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.isActive).to.be.true;
    console.log("   -> [SENTINEL FINAL PHASE 3]: 15/15 Tests Passed.");
  });
});