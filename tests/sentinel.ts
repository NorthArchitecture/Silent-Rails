import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { Sentinel } from "../target/types/sentinel";
import{
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";

describe("sentinel", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Sentinel as Program<Sentinel>;

  let northMint: anchor.web3.PublicKey;
  let mintAuthority: anchor.web3.Keypair;

  before(async () => {
    mintAuthority = anchor.web3.Keypair.generate();
    await fundAccount(mintAuthority.publicKey);

    northMint = await createMint(
      provider.connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      9
    );
  });

  async function fundAccount(publicKey: anchor.web3.PublicKey) {
    const airdropSig = await provider.connection.requestAirdrop(
      publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    const latestBlockhash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction(
      {
        signature: airdropSig,
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      },
      "confirmed"
    );

    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  async function setupTokenAccount(owner: anchor.web3.Keypair) {
    const tokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      mintAuthority,
      northMint,
      owner.publicKey
    );

    await mintTo(
      provider.connection,
      mintAuthority,
      northMint,
      tokenAccount.address,
      mintAuthority,
      1000 * 1_000_000_000
    );

    return tokenAccount.address;
  }

  it("initialize_rail", async () => {
    const authority = anchor.web3.Keypair.generate();
    await fundAccount(authority.publicKey);

    const authorityTokenAccount = await setupTokenAccount(authority);

    const [railPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("rail"), authority.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .initializeRail(1, 2)
      .accounts({
        rail: railPda,
        authority: authority.publicKey,
        authorityTokenAccount: authorityTokenAccount,
        northMint: northMint,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const account = await (program.account as any).railState.fetch(railPda);
    expect(account.isActive).to.be.true;
  });

  it("create_handshake_and_seal", async () => {
    const authority = anchor.web3.Keypair.generate();
    await fundAccount(authority.publicKey);

    const authorityTokenAccount = await setupTokenAccount(authority);

    const [railPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("rail"), authority.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .initializeRail(1, 2)
      .accounts({
        rail: railPda,
        authority: authority.publicKey,
        authorityTokenAccount: authorityTokenAccount,
        northMint: northMint,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const commitment = Array.from(new Uint8Array(32).fill(1));
    const nullifierHash = Array.from(new Uint8Array(32).fill(2));

    await program.methods
      .createHandshake(commitment, nullifierHash)
      .accounts({
        rail: railPda,
        payer: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const auditSeal = Array.from(new Uint8Array(32).fill(9));

    await program.methods
      .sealRail(auditSeal)
      .accounts({
        rail: railPda,
        authority: authority.publicKey,
      })
      .signers([authority])
      .rpc();

    const account = await (program.account as any).railState.fetch(railPda);
    expect(account.isSealed).to.be.true;
  });

  it("security_unauthorized", async () => {
    const authority = anchor.web3.Keypair.generate();
    await fundAccount(authority.publicKey);

    const authorityTokenAccount = await setupTokenAccount(authority);

    const [railPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("rail"), authority.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .initializeRail(1, 2)
      .accounts({
        rail: railPda,
        authority: authority.publicKey,
        authorityTokenAccount: authorityTokenAccount,
        northMint: northMint,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const unauthorizedUser = anchor.web3.Keypair.generate();

    try {
      await program.methods
        .sealRail(Array.from(new Uint8Array(32).fill(0)))
        .accounts({
          rail: railPda,
          authority: unauthorizedUser.publicKey,
        })
        .signers([unauthorizedUser])
        .rpc();
      expect.fail();
    } catch (err: any) {
      expect(err.toString()).to.include("6001");
    }
  });

  it("should_fail_without_north_tokens", async () => {
    const authority = anchor.web3.Keypair.generate();
    await fundAccount(authority.publicKey);

    const emptyTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      mintAuthority,
      northMint,
      authority.publicKey
    );

    const [railPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("rail"), authority.publicKey.toBuffer()],
      program.programId
    );

    try {
      await program.methods
        .initializeRail(1, 2)
        .accounts({
          rail: railPda,
          authority: authority.publicKey,
          authorityTokenAccount: emptyTokenAccount.address,
          northMint: northMint,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([authority])
        .rpc();
      expect.fail();
    } catch (err: any) {
      expect(err.toString()).to.include("6000");
    }
  });
});