import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { Sentinel } from "../target/types/sentinel";

describe("sentinel", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Sentinel as Program<Sentinel>;

  async function fundAccount(publicKey: anchor.web3.PublicKey) {
    const airdropSig = await provider.connection.requestAirdrop(
      publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    const latestBlockhash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      signature: airdropSig,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    }, "confirmed");
    
    await new Promise(resolve => setTimeout(resolve, 1000));
  }

  it("initialize_rail", async () => {
    const authority = anchor.web3.Keypair.generate();
    await fundAccount(authority.publicKey);

    const institutionType = 1;
    const complianceLevel = 2;
    
    const [railPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        anchor.utils.bytes.utf8.encode("rail"),
        authority.publicKey.toBuffer()
      ],
      program.programId
    );

    await program.methods
      .initializeRail(institutionType, complianceLevel)
      .accounts({
        rail: railPda,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .signers([authority])
      .rpc();

    const account = await program.account.railState.fetch(railPda);
    expect(account.isActive).to.be.true;
  });

  it("create_handshake_and_seal", async () => {
    const authority = anchor.web3.Keypair.generate();
    await fundAccount(authority.publicKey);

    const [railPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        anchor.utils.bytes.utf8.encode("rail"),
        authority.publicKey.toBuffer()
      ],
      program.programId
    );

    await program.methods
      .initializeRail(1, 2)
      .accounts({
        rail: railPda,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .signers([authority])
      .rpc();

    const commitment = Array.from(new Uint8Array(32).fill(1));
    const nullifierHash = Array.from(new Uint8Array(32).fill(2));

    await program.methods
      .createHandshake(commitment, nullifierHash)
      .accounts({
        rail: railPda,
        payer: authority.publicKey,
        authority: authority.publicKey,
      } as any)
      .signers([authority])
      .rpc();

    const auditSeal = Array.from(new Uint8Array(32).fill(9));

    await program.methods
      .sealRail(auditSeal)
      .accounts({
        rail: railPda,
        authority: authority.publicKey,
      } as any)
      .signers([authority])
      .rpc();

    const railAccount = await program.account.railState.fetch(railPda);
    expect(railAccount.isSealed).to.be.true;
  });

  it("security_unauthorized", async () => {
    const authority = anchor.web3.Keypair.generate();
    await fundAccount(authority.publicKey);

    const [railPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        anchor.utils.bytes.utf8.encode("rail"),
        authority.publicKey.toBuffer()
      ],
      program.programId
    );

    await program.methods
      .initializeRail(1, 2)
      .accounts({
        rail: railPda,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .signers([authority])
      .rpc();

    const unauthorizedUser = anchor.web3.Keypair.generate();

    try {
      await program.methods
        .sealRail(Array.from(new Uint8Array(32).fill(0)))
        .accounts({
          rail: railPda,
          authority: unauthorizedUser.publicKey,
        } as any)
        .signers([unauthorizedUser])
        .rpc();
      expect.fail("Should have thrown unauthorized error");
    } catch (err: any) {
      expect(err.toString()).to.include("AnchorError");
    }
  });
});