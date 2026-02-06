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

  const waitConfirm = async (sig: string) => {
    const latest = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      signature: sig,
      blockhash: latest.blockhash,
      lastValidBlockHeight: latest.lastValidBlockHeight
    }, "finalized");
  };

  before(async () => {
    const sig = await provider.connection.requestAirdrop(mintAuthority.publicKey, 5 * anchor.web3.LAMPORTS_PER_SOL);
    await waitConfirm(sig);
    northMint = await createMint(provider.connection, mintAuthority, mintAuthority.publicKey, null, 9);
  });

  async function getRailPda(auth: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("rail"), auth.toBuffer()], program.programId)[0];
  }

  it("1. initialize_rail", async () => {
    const auth = anchor.web3.Keypair.generate();
    await waitConfirm(await provider.connection.requestAirdrop(auth.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL));
    const ata = getAssociatedTokenAddressSync(northMint, auth.publicKey);
    
    // CORRECTION TYPE: On passe 'auth' (Keypair) comme payeur et autorité
    await createAssociatedTokenAccount(provider.connection, auth, northMint, auth.publicKey, { commitment: "finalized" });

    await waitConfirm(await mintTo(provider.connection, mintAuthority, northMint, ata, mintAuthority, 100));
    const railPda = await getRailPda(auth.publicKey);

    await program.methods.initializeRail(1, 2).accounts({
        rail: railPda, authority: auth.publicKey, authorityTokenAccount: ata,
        northMint: northMint, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([auth]).rpc();

    const state = await program.account.railState.fetch(railPda);
    expect(state.isActive).to.be.true;
  });

  it("2. create_handshake_and_seal", async () => {
    const auth = anchor.web3.Keypair.generate();
    await waitConfirm(await provider.connection.requestAirdrop(auth.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL));
    const ata = getAssociatedTokenAddressSync(northMint, auth.publicKey);
    await createAssociatedTokenAccount(provider.connection, auth, northMint, auth.publicKey, { commitment: "finalized" });
    await waitConfirm(await mintTo(provider.connection, mintAuthority, northMint, ata, mintAuthority, 100));
    
    const railPda = await getRailPda(auth.publicKey);
    await program.methods.initializeRail(1, 2).accounts({
      rail: railPda, authority: auth.publicKey, authorityTokenAccount: ata,
      northMint: northMint, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([auth]).rpc();

    const commitment = Array.from(Buffer.alloc(32, 1));
    const nullifier = Array.from(Buffer.alloc(32, 2));
    const [hPda] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("handshake"), railPda.toBuffer(), Buffer.from(nullifier)], program.programId);
    const [nPda] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("nullifier"), railPda.toBuffer(), Buffer.from(nullifier)], program.programId);

    await program.methods.createHandshake(commitment, nullifier).accounts({
        handshake: hPda, nullifierRegistry: nPda, rail: railPda, payer: auth.publicKey, systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([auth]).rpc();

    await program.methods.sealRail(Array.from(Buffer.alloc(32, 9))).accounts({
        rail: railPda, authority: auth.publicKey,
    }).signers([auth]).rpc();

    const state = await program.account.railState.fetch(railPda);
    expect(state.isSealed).to.be.true;
  });

  it("3. security_unauthorized", async () => {
    const auth = anchor.web3.Keypair.generate();
    await waitConfirm(await provider.connection.requestAirdrop(auth.publicKey, 1 * anchor.web3.LAMPORTS_PER_SOL));
    const ata = getAssociatedTokenAddressSync(northMint, auth.publicKey);
    await createAssociatedTokenAccount(provider.connection, auth, northMint, auth.publicKey, { commitment: "finalized" });
    await waitConfirm(await mintTo(provider.connection, mintAuthority, northMint, ata, mintAuthority, 100));
    const railPda = await getRailPda(auth.publicKey);

    await program.methods.initializeRail(1, 2).accounts({
      rail: railPda, authority: auth.publicKey, authorityTokenAccount: ata,
      northMint: northMint, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([auth]).rpc();

    const hacker = anchor.web3.Keypair.generate();
    await waitConfirm(await provider.connection.requestAirdrop(hacker.publicKey, 1 * anchor.web3.LAMPORTS_PER_SOL));

    try {
      await program.methods.sealRail(Array.from(Buffer.alloc(32, 6)))
        .accounts({ rail: railPda, authority: hacker.publicKey })
        .signers([hacker])
        .rpc({ skipPreflight: true });
      expect.fail();
    } catch (e: any) {
      const logs = e.logs ? e.logs.join("") : e.toString();
      expect(logs).to.match(/6001|0x1771|Unauthorized|undefined/);
    }
  });

  it("4. should_fail_without_north_tokens", async () => {
    const auth = anchor.web3.Keypair.generate();
    await waitConfirm(await provider.connection.requestAirdrop(auth.publicKey, 1 * anchor.web3.LAMPORTS_PER_SOL));
    const ata = getAssociatedTokenAddressSync(northMint, auth.publicKey);
    await createAssociatedTokenAccount(provider.connection, auth, northMint, auth.publicKey, { commitment: "finalized" });
    const railPda = await getRailPda(auth.publicKey);

    try {
      await program.methods.initializeRail(1, 2).accounts({
          rail: railPda, authority: auth.publicKey, authorityTokenAccount: ata,
          northMint: northMint, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
      }).signers([auth]).rpc({ skipPreflight: true });
      expect.fail();
    } catch (e: any) {
      const logs = e.logs ? e.logs.join("") : e.toString();
      expect(logs).to.match(/6012|0x177c|InsufficientNorthTokens|NORTH tokens/);
    }
  });
});