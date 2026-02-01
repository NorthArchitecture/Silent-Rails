import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Sentinel } from "../target/types/sentinel";
import { expect } from "chai";

describe("silent-rails-infrastructure", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Sentinel as Program<Sentinel>;
  const authority = provider.wallet;

  // Generate a keypair for the privacy rail
  const railKeypair = anchor.web3.Keypair.generate();

  it("1. Initialize Privacy Handshake (PDA Validation)", async () => {
    const fragmentId = new anchor.BN(1);
    const zkEvidence = Array(32).fill(1); // Mock ZK-Evidence
    
    // Derive PDA (must match Rust seeds)
    const [handshakePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("handshake"), 
        authority.publicKey.toBuffer(), 
        fragmentId.toArrayLike(Buffer, "le", 8)
      ],
      program.programId
    );

    await program.methods
      .initializeHandshake(fragmentId, zkEvidence)
      .accounts({
        handshake: handshakePda,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    
    const account = await program.account.handshakeState.fetch(handshakePda);
    expect(account.isActive).to.be.true;
    console.log("✅ Handshake PDA verified at:", handshakePda.toBase58());
  });

  it("2. Open Institutional Privacy Rail", async () => {
    await program.methods
      .openPrivacyRail()
      .accounts({
        rail: railKeypair.publicKey,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([railKeypair])
      .rpc();

    const account = await program.account.railState.fetch(railKeypair.publicKey);
    expect(account.isSealed).to.be.false;
    console.log("✅ Privacy Rail successfully opened.");
  });

  it("3. Seal Rail with Audit Evidence", async () => {
    const auditSeal = Array(32).fill(7); // Mock Audit Seal

    await program.methods
      .sealPrivacyRail(auditSeal)
      .accounts({
        rail: railKeypair.publicKey,
        authority: authority.publicKey,
      })
      .rpc();

    const account = await program.account.railState.fetch(railKeypair.publicKey);
    expect(account.isSealed).to.be.true;
    console.log("✅ Rail sealed and secured for production.");
  });
});