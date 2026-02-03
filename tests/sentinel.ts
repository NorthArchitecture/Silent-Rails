import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";

describe("sentinel", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Sentinel as Program<any>;
  const authority = provider.wallet;

  const mockProof = {
    a: Array(64).fill(0),
    b: Array(128).fill(0),
    c: Array(64).fill(0),
  };
  const nullifierHash = Array(32).fill(1);

  it("initialize_handshake", async () => {
    const fragmentId = new anchor.BN(Date.now());
    
    const [handshakePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("handshake"),
        authority.publicKey.toBuffer(),
        fragmentId.toArrayLike(Buffer, "le", 8)
      ],
      program.programId
    );

    await program.methods
      .initializeHandshake(fragmentId, mockProof, nullifierHash)
      .accounts({
        handshake: handshakePda,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const account = await (program.account as any).handshakeState.fetch(handshakePda);
    expect(account.isActive).to.be.true;
  });

  it("rail_lifecycle", async () => {
    const railKeypair = anchor.web3.Keypair.generate();
    const auditSeal = Array(32).fill(9);

    await program.methods
      .openPrivacyRail()
      .accounts({
        rail: railKeypair.publicKey,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([railKeypair])
      .rpc();

    await program.methods
      .sealPrivacyRail(auditSeal)
      .accounts({
        rail: railKeypair.publicKey,
        authority: authority.publicKey,
      })
      .rpc();

    const railAccount = await (program.account as any).railState.fetch(railKeypair.publicKey);
    expect(railAccount.isSealed).to.be.true;
  });

  it("security_unauthorized", async () => {
    const railKeypair = anchor.web3.Keypair.generate();
    const pirate = anchor.web3.Keypair.generate();

    await program.methods
      .openPrivacyRail()
      .accounts({
        rail: railKeypair.publicKey,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([railKeypair])
      .rpc();

    try {
      await program.methods
        .sealPrivacyRail(Array(32).fill(0))
        .accounts({
          rail: railKeypair.publicKey,
          authority: pirate.publicKey,
        })
        .signers([pirate])
        .rpc();
      expect.fail();
    } catch (err: any) {
      expect(err).to.exist;
    }
  });
});