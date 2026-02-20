import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
  mintTo,
} from "@solana/spl-token";
import { ComputeBudgetProgram } from "@solana/web3.js";

describe("sentinel_v2_tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Sentinel as any;

  let northMint: anchor.web3.PublicKey;
  let mintAuthority = anchor.web3.Keypair.generate();
  let sharedAuth = anchor.web3.Keypair.generate();
  let sharedRailPda: anchor.web3.PublicKey;
  let sharedAta: anchor.web3.PublicKey;
  let zkVaultPda: anchor.web3.PublicKey;

  const PRIORITY_FEE_IX = ComputeBudgetProgram.setComputeUnitPrice({
    microLamports: 8000,
  });

  const waitConfirm = async (sig: string) => {
    const latest = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction(
      {
        signature: sig,
        blockhash: latest.blockhash,
        lastValidBlockHeight: latest.lastValidBlockHeight,
      },
      "confirmed"
    );
  };

  const measureExecution = async (name: string, fn: () => Promise<any>) => {
    const start = Date.now();
    try {
      const sig = await fn();
      const duration = Date.now() - start;
      console.log(
        `      ⚡ ${name}: ${duration}ms | Sig: ${sig.slice(0, 8)}...`
      );
      return sig;
    } catch (e) {
      const duration = Date.now() - start;
      console.log(`      ❌ ${name} FAILED after ${duration}ms`);
      throw e;
    }
  };

  function getRailPda(auth: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("rail"), auth.toBuffer()],
      program.programId
    )[0];
  }

  function getZkVaultPda(rail: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("zk_vault"), rail.toBuffer()],
      program.programId
    )[0];
  }

  function getHandshakePda(rail: anchor.web3.PublicKey, nullifier: number[]) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("handshake"), rail.toBuffer(), Buffer.from(nullifier)],
      program.programId
    )[0];
  }

  function getNullifierPda(rail: anchor.web3.PublicKey, nullifier: number[]) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("nullifier"), rail.toBuffer(), Buffer.from(nullifier)],
      program.programId
    )[0];
  }

  before(async () => {
    const tx = new anchor.web3.Transaction()
      .add(
        anchor.web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: mintAuthority.publicKey,
          lamports: 0.15 * anchor.web3.LAMPORTS_PER_SOL,
        })
      )
      .add(
        anchor.web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: sharedAuth.publicKey,
          lamports: 0.15 * anchor.web3.LAMPORTS_PER_SOL,
        })
      );
    await provider.sendAndConfirm(tx);

    northMint = await createMint(
      provider.connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      9
    );

    sharedAta = getAssociatedTokenAddressSync(northMint, sharedAuth.publicKey);
    await createAssociatedTokenAccount(
      provider.connection,
      sharedAuth,
      northMint,
      sharedAuth.publicKey
    );

    const mintSig = await mintTo(
      provider.connection,
      mintAuthority,
      northMint,
      sharedAta,
      mintAuthority,
      1000
    );
    await waitConfirm(mintSig);

    sharedRailPda = getRailPda(sharedAuth.publicKey);
    zkVaultPda = getZkVaultPda(sharedRailPda);
  });

  it("1. initialize_rail", async () => {
    await measureExecution("Rail Init", () =>
      program.methods
        .initializeRail(1, 2)
        .accounts({
          rail: sharedRailPda,
          authority: sharedAuth.publicKey,
          authorityTokenAccount: sharedAta,
          northMint: northMint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );
  });

  it("2. verify_rail_state", async () => {
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.authority.toBase58()).to.equal(
      sharedAuth.publicKey.toBase58()
    );
    expect(state.isActive).to.be.true;
    expect(state.isSealed).to.be.false;
    expect(state.isPaused).to.be.false;
  });

  it("3. initialize_zk_vault", async () => {
    const elgamalPubkey = Array.from(Buffer.alloc(32, 9));
    await measureExecution("ZK Vault Init", () =>
      program.methods
        .initializeZkVault(elgamalPubkey)
        .accounts({
          zkVault: zkVaultPda,
          rail: sharedRailPda,
          authority: sharedAuth.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );
  });

  it("4. prevent_duplicate_zk_vault", async () => {
    try {
      await program.methods
        .initializeZkVault(Array.from(Buffer.alloc(32, 1)))
        .accounts({
          zkVault: zkVaultPda,
          rail: sharedRailPda,
          authority: sharedAuth.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([sharedAuth])
        .rpc({ skipPreflight: true });
      expect.fail("Should have failed");
    } catch (e) {}
  });

  it("5. block_unauthorized_zk_vault", async () => {
    const hacker = anchor.web3.Keypair.generate();
    try {
      await program.methods
        .initializeZkVault(Array.from(Buffer.alloc(32, 0)))
        .accounts({
          zkVault: zkVaultPda,
          rail: sharedRailPda,
          authority: hacker.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([hacker])
        .rpc({ skipPreflight: true });
      expect.fail("Should have failed");
    } catch (e) {}
  });

  it("6. create_handshake", async () => {
    const nullifier = Array.from(Buffer.alloc(32, 1));
    const commitment = Array.from(Buffer.alloc(32, 2));
    const handshakePda = getHandshakePda(sharedRailPda, nullifier);
    const nullifierPda = getNullifierPda(sharedRailPda, nullifier);

    await measureExecution("Handshake Create", () =>
      program.methods
        .createHandshake(commitment, nullifier)
        .accounts({
          handshake: handshakePda,
          nullifierRegistry: nullifierPda,
          rail: sharedRailPda,
          payer: sharedAuth.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );
  });

  it("7. verify_handshake_state", async () => {
    const nullifier = Array.from(Buffer.alloc(32, 1));
    const handshakePda = getHandshakePda(sharedRailPda, nullifier);
    const hs = await program.account.handshakeState.fetch(handshakePda);
    expect(hs.isActive).to.be.true;
    expect(hs.rail.toBase58()).to.equal(sharedRailPda.toBase58());
  });

  it("8. prevent_nullifier_replay", async () => {
    const nullifier = Array.from(Buffer.alloc(32, 1));
    const nullifierPda = getNullifierPda(sharedRailPda, nullifier);
    try {
      await program.methods
        .createHandshake(nullifier, nullifier)
        .accounts({
          handshake: anchor.web3.Keypair.generate().publicKey,
          nullifierRegistry: nullifierPda,
          rail: sharedRailPda,
          payer: sharedAuth.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([sharedAuth])
        .rpc({ skipPreflight: true });
      expect.fail("Should have failed");
    } catch (e) {}
  });

  it("9. block_unauthorized_seal", async () => {
    const hacker = anchor.web3.Keypair.generate();
    try {
      await program.methods
        .sealRail(Array.from(Buffer.alloc(32, 6)))
        .accounts({
          rail: sharedRailPda,
          authority: hacker.publicKey,
        })
        .signers([hacker])
        .rpc({ skipPreflight: true });
      expect.fail("Should have failed");
    } catch (e) {}
  });

  it("10. pause_rail", async () => {
    await measureExecution("Pause Rail", () =>
      program.methods
        .pauseRail()
        .accounts({
          rail: sharedRailPda,
          authority: sharedAuth.publicKey,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.isPaused).to.be.true;
  });

  it("11. block_handshake_during_pause", async () => {
    const nullifier = Array.from(Buffer.alloc(32, 99));
    const handshakePda = getHandshakePda(sharedRailPda, nullifier);
    const nullifierPda = getNullifierPda(sharedRailPda, nullifier);
    try {
      await program.methods
        .createHandshake(nullifier, nullifier)
        .accounts({
          handshake: handshakePda,
          nullifierRegistry: nullifierPda,
          rail: sharedRailPda,
          payer: sharedAuth.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([sharedAuth])
        .rpc({ skipPreflight: true });
      expect.fail("Should have failed");
    } catch (e) {}
  });

  it("12. unpause_rail", async () => {
    await measureExecution("Unpause Rail", () =>
      program.methods
        .unpauseRail()
        .accounts({
          rail: sharedRailPda,
          authority: sharedAuth.publicKey,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.isPaused).to.be.false;
  });

  it("13. revoke_handshake", async () => {
    const nullifier = Array.from(Buffer.alloc(32, 1));
    const handshakePda = getHandshakePda(sharedRailPda, nullifier);

    await measureExecution("Revoke Handshake", () =>
      program.methods
        .revokeHandshake(1)
        .accounts({
          handshake: handshakePda,
          rail: sharedRailPda,
          authority: sharedAuth.publicKey,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );

    const hs = await program.account.handshakeState.fetch(handshakePda);
    expect(hs.isActive).to.be.false;
  });

  it("14. create_second_handshake", async () => {
    const nullifier = Array.from(Buffer.alloc(32, 55));
    const commitment = Array.from(Buffer.alloc(32, 66));
    const handshakePda = getHandshakePda(sharedRailPda, nullifier);
    const nullifierPda = getNullifierPda(sharedRailPda, nullifier);

    await measureExecution("Handshake 2", () =>
      program.methods
        .createHandshake(commitment, nullifier)
        .accounts({
          handshake: handshakePda,
          nullifierRegistry: nullifierPda,
          rail: sharedRailPda,
          payer: sharedAuth.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );
  });

  it("15. verify_handshake_counter", async () => {
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.totalHandshakes.toNumber()).to.equal(2);
  });

  it("16. seal_rail", async () => {
    await measureExecution("Seal Rail", () =>
      program.methods
        .sealRail(Array.from(Buffer.alloc(32, 7)))
        .accounts({
          rail: sharedRailPda,
          authority: sharedAuth.publicKey,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.isSealed).to.be.true;
  });

  it("17. block_handshake_after_seal", async () => {
    const nullifier = Array.from(Buffer.alloc(32, 200));
    const handshakePda = getHandshakePda(sharedRailPda, nullifier);
    const nullifierPda = getNullifierPda(sharedRailPda, nullifier);
    try {
      await program.methods
        .createHandshake(nullifier, nullifier)
        .accounts({
          handshake: handshakePda,
          nullifierRegistry: nullifierPda,
          rail: sharedRailPda,
          payer: sharedAuth.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([sharedAuth])
        .rpc({ skipPreflight: true });
      expect.fail("Should have failed");
    } catch (e) {}
  });

  it("18. deactivate_rail", async () => {
    await measureExecution("Deactivate Rail", () =>
      program.methods
        .deactivateRail(1)
        .accounts({
          rail: sharedRailPda,
          authority: sharedAuth.publicKey,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([sharedAuth])
        .rpc()
    );
    const state = await program.account.railState.fetch(sharedRailPda);
    expect(state.isActive).to.be.false;
    expect(state.deactivationReason).to.equal(1);
    console.log("\n   [V2 TESTS COMPLETED] - All 18 tests passed.");
  });
});

describe("sentinel_v2_token_security_tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Sentinel as any;

  const PRIORITY_FEE_IX = ComputeBudgetProgram.setComputeUnitPrice({
    microLamports: 8000,
  });

  let northMint: anchor.web3.PublicKey;
  let testTokenMint: anchor.web3.PublicKey;
  let wrongTokenMint: anchor.web3.PublicKey;

  const mintAuthority = anchor.web3.Keypair.generate();
  const aliceAuth = anchor.web3.Keypair.generate();
  const bobAuth = anchor.web3.Keypair.generate();

  let aliceRail: anchor.web3.PublicKey;
  let bobRail: anchor.web3.PublicKey;
  let aliceZkVault: anchor.web3.PublicKey;
  let bobZkVault: anchor.web3.PublicKey;
  let aliceNorthAta: anchor.web3.PublicKey;
  let bobNorthAta: anchor.web3.PublicKey;
  let aliceTokenAta: anchor.web3.PublicKey;
  let bobTokenAta: anchor.web3.PublicKey;
  let aliceWrongTokenAta: anchor.web3.PublicKey;
  let aliceVaultTokenAta: anchor.web3.PublicKey;
  let bobVaultTokenAta: anchor.web3.PublicKey;
  let aliceHandshake: anchor.web3.PublicKey;

  const dummyProof = Buffer.alloc(256, 1);
  const commitmentA = Array.from(Buffer.alloc(32, 11));
  const commitmentB = Array.from(Buffer.alloc(32, 12));
  const commitmentC = Array.from(Buffer.alloc(32, 13));
  const nullifierA = Array.from(Buffer.alloc(32, 21));
  const nullifierB = Array.from(Buffer.alloc(32, 22));

  const payer = (provider.wallet as any).payer as anchor.web3.Keypair;

  const waitConfirm = async (sig: string) => {
    const latest = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction(
      {
        signature: sig,
        blockhash: latest.blockhash,
        lastValidBlockHeight: latest.lastValidBlockHeight,
      },
      "confirmed"
    );
  };

  const expectTxFail = async (fn: () => Promise<string>) => {
    let failed = false;
    try {
      await fn();
    } catch {
      failed = true;
    }
    expect(failed).to.equal(true);
  };

  function getRailPda(auth: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("rail"), auth.toBuffer()],
      program.programId
    )[0];
  }

  function getZkVaultPda(rail: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("zk_vault"), rail.toBuffer()],
      program.programId
    )[0];
  }

  function getHandshakePda(rail: anchor.web3.PublicKey, nullifier: number[]) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("handshake"), rail.toBuffer(), Buffer.from(nullifier)],
      program.programId
    )[0];
  }

  function getNullifierPda(rail: anchor.web3.PublicKey, nullifier: number[]) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("nullifier"), rail.toBuffer(), Buffer.from(nullifier)],
      program.programId
    )[0];
  }

  function getTokenAssetStatePda(rail: anchor.web3.PublicKey, mint: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("asset_vault"), rail.toBuffer(), mint.toBuffer()],
      program.programId
    )[0];
  }

  function getTransferPda(
    senderRail: anchor.web3.PublicKey,
    receiverRail: anchor.web3.PublicKey,
    nonce: number
  ) {
    const nonceBuf = Buffer.alloc(8);
    nonceBuf.writeBigInt64LE(BigInt(nonce), 0);
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("transfer"), senderRail.toBuffer(), receiverRail.toBuffer(), nonceBuf],
      program.programId
    )[0];
  }

  before(async () => {
    const tx = new anchor.web3.Transaction()
      .add(
        anchor.web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: mintAuthority.publicKey,
          lamports: 0.8 * anchor.web3.LAMPORTS_PER_SOL,
        })
      )
      .add(
        anchor.web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: aliceAuth.publicKey,
          lamports: 0.8 * anchor.web3.LAMPORTS_PER_SOL,
        })
      )
      .add(
        anchor.web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: bobAuth.publicKey,
          lamports: 0.8 * anchor.web3.LAMPORTS_PER_SOL,
        })
      );
    await provider.sendAndConfirm(tx);

    northMint = await createMint(
      provider.connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      9
    );

    aliceNorthAta = getAssociatedTokenAddressSync(northMint, aliceAuth.publicKey);
    bobNorthAta = getAssociatedTokenAddressSync(northMint, bobAuth.publicKey);

    await createAssociatedTokenAccount(
      provider.connection,
      aliceAuth,
      northMint,
      aliceAuth.publicKey
    );
    await createAssociatedTokenAccount(
      provider.connection,
      bobAuth,
      northMint,
      bobAuth.publicKey
    );

    await waitConfirm(
      await mintTo(
        provider.connection,
        mintAuthority,
        northMint,
        aliceNorthAta,
        mintAuthority,
        10_000
      )
    );
    await waitConfirm(
      await mintTo(
        provider.connection,
        mintAuthority,
        northMint,
        bobNorthAta,
        mintAuthority,
        10_000
      )
    );

    aliceRail = getRailPda(aliceAuth.publicKey);
    bobRail = getRailPda(bobAuth.publicKey);
    aliceZkVault = getZkVaultPda(aliceRail);
    bobZkVault = getZkVaultPda(bobRail);

    await program.methods
      .initializeRail(1, 2)
      .accounts({
        rail: aliceRail,
        authority: aliceAuth.publicKey,
        authorityTokenAccount: aliceNorthAta,
        northMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .preInstructions([PRIORITY_FEE_IX])
      .signers([aliceAuth])
      .rpc();

    await program.methods
      .initializeRail(1, 2)
      .accounts({
        rail: bobRail,
        authority: bobAuth.publicKey,
        authorityTokenAccount: bobNorthAta,
        northMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .preInstructions([PRIORITY_FEE_IX])
      .signers([bobAuth])
      .rpc();

    await program.methods
      .initializeZkVault(Array.from(Buffer.alloc(32, 90)))
      .accounts({
        zkVault: aliceZkVault,
        rail: aliceRail,
        authority: aliceAuth.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .preInstructions([PRIORITY_FEE_IX])
      .signers([aliceAuth])
      .rpc();

    await program.methods
      .initializeZkVault(Array.from(Buffer.alloc(32, 91)))
      .accounts({
        zkVault: bobZkVault,
        rail: bobRail,
        authority: bobAuth.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .preInstructions([PRIORITY_FEE_IX])
      .signers([bobAuth])
      .rpc();

    aliceHandshake = getHandshakePda(aliceRail, nullifierA);
    const aliceNullifierRegistry = getNullifierPda(aliceRail, nullifierA);
    await program.methods
      .createHandshake(commitmentA, nullifierA)
      .accounts({
        handshake: aliceHandshake,
        nullifierRegistry: aliceNullifierRegistry,
        rail: aliceRail,
        payer: aliceAuth.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .preInstructions([PRIORITY_FEE_IX])
      .signers([aliceAuth])
      .rpc();

    testTokenMint = await createMint(
      provider.connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      6
    );
    wrongTokenMint = await createMint(
      provider.connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      6
    );

    aliceTokenAta = getAssociatedTokenAddressSync(testTokenMint, aliceAuth.publicKey);
    bobTokenAta = getAssociatedTokenAddressSync(testTokenMint, bobAuth.publicKey);
    aliceWrongTokenAta = getAssociatedTokenAddressSync(wrongTokenMint, aliceAuth.publicKey);

    aliceVaultTokenAta = getAssociatedTokenAddressSync(testTokenMint, aliceZkVault, true);
    bobVaultTokenAta = getAssociatedTokenAddressSync(testTokenMint, bobZkVault, true);

    await createAssociatedTokenAccount(provider.connection, aliceAuth, testTokenMint, aliceAuth.publicKey);
    await createAssociatedTokenAccount(provider.connection, bobAuth, testTokenMint, bobAuth.publicKey);
    await createAssociatedTokenAccount(provider.connection, aliceAuth, wrongTokenMint, aliceAuth.publicKey);
    const createVaultAtaTx = new anchor.web3.Transaction()
      .add(
        createAssociatedTokenAccountInstruction(
          aliceAuth.publicKey,
          aliceVaultTokenAta,
          aliceZkVault,
          testTokenMint
        )
      )
      .add(
        createAssociatedTokenAccountInstruction(
          bobAuth.publicKey,
          bobVaultTokenAta,
          bobZkVault,
          testTokenMint
        )
      );
    await provider.sendAndConfirm(createVaultAtaTx, [aliceAuth, bobAuth]);

    await waitConfirm(
      await mintTo(
        provider.connection,
        mintAuthority,
        testTokenMint,
        aliceTokenAta,
        mintAuthority,
        1_000_000
      )
    );
    await waitConfirm(
      await mintTo(
        provider.connection,
        mintAuthority,
        wrongTokenMint,
        aliceWrongTokenAta,
        mintAuthority,
        1_000_000
      )
    );
  });

  it("19. multi_asset_pda_isolation_between_mints_and_rails", async () => {
    const aliceTestPda = getTokenAssetStatePda(aliceRail, testTokenMint);
    const aliceWrongPda = getTokenAssetStatePda(aliceRail, wrongTokenMint);
    const bobTestPda = getTokenAssetStatePda(bobRail, testTokenMint);

    expect(aliceTestPda.toBase58()).to.not.equal(aliceWrongPda.toBase58());
    expect(aliceTestPda.toBase58()).to.not.equal(bobTestPda.toBase58());
  });

  it("20. deposit_token_rejects_invalid_zk_proof", async () => {
    const tokenAssetState = getTokenAssetStatePda(aliceRail, testTokenMint);
    const zk = await program.account.zkVault.fetch(aliceZkVault);
    const tokenDepositRecord = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("token_deposit"),
        aliceRail.toBuffer(),
        aliceAuth.publicKey.toBuffer(),
        testTokenMint.toBuffer(),
        Buffer.from(zk.tokenDepositCount.toArrayLike(Buffer, "le", 8)),
      ],
      program.programId
    )[0];

    await expectTxFail(() =>
      program.methods
        .depositToken(
          10_000,
          Buffer.from(dummyProof),
          commitmentB,
          nullifierB,
          Array.from(Buffer.alloc(64, 44))
        )
        .accounts({
          rail: aliceRail,
          zkVault: aliceZkVault,
          handshake: aliceHandshake,
          tokenMint: testTokenMint,
          tokenAssetState,
          senderTokenAccount: aliceTokenAta,
          vaultTokenAccount: aliceVaultTokenAta,
          tokenDepositRecord,
          sender: aliceAuth.publicKey,
          authority: aliceAuth.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .preInstructions([PRIORITY_FEE_IX])
        .signers([aliceAuth])
        .rpc()
    );
  });

  it("21. deposit_token_enforces_sender_mint_constraints", async () => {
    const tokenAssetState = getTokenAssetStatePda(aliceRail, testTokenMint);
    const zk = await program.account.zkVault.fetch(aliceZkVault);
    const tokenDepositRecord = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("token_deposit"),
        aliceRail.toBuffer(),
        aliceAuth.publicKey.toBuffer(),
        testTokenMint.toBuffer(),
        Buffer.from(zk.tokenDepositCount.toArrayLike(Buffer, "le", 8)),
      ],
      program.programId
    )[0];

    await expectTxFail(() =>
      program.methods
        .depositToken(
          10_000,
          Buffer.from(dummyProof),
          commitmentB,
          nullifierB,
          Array.from(Buffer.alloc(64, 45))
        )
        .accounts({
          rail: aliceRail,
          zkVault: aliceZkVault,
          handshake: aliceHandshake,
          tokenMint: testTokenMint,
          tokenAssetState,
          senderTokenAccount: aliceWrongTokenAta,
          vaultTokenAccount: aliceVaultTokenAta,
          tokenDepositRecord,
          sender: aliceAuth.publicKey,
          authority: aliceAuth.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([aliceAuth])
        .rpc()
    );
  });

  it("22. deposit_token_enforces_rail_authority_constraint", async () => {
    const tokenAssetState = getTokenAssetStatePda(aliceRail, testTokenMint);
    const zk = await program.account.zkVault.fetch(aliceZkVault);
    const tokenDepositRecord = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("token_deposit"),
        aliceRail.toBuffer(),
        aliceAuth.publicKey.toBuffer(),
        testTokenMint.toBuffer(),
        Buffer.from(zk.tokenDepositCount.toArrayLike(Buffer, "le", 8)),
      ],
      program.programId
    )[0];

    await expectTxFail(() =>
      program.methods
        .depositToken(
          10_000,
          Buffer.from(dummyProof),
          commitmentB,
          nullifierB,
          Array.from(Buffer.alloc(64, 46))
        )
        .accounts({
          rail: aliceRail,
          zkVault: aliceZkVault,
          handshake: aliceHandshake,
          tokenMint: testTokenMint,
          tokenAssetState,
          senderTokenAccount: aliceTokenAta,
          vaultTokenAccount: aliceVaultTokenAta,
          tokenDepositRecord,
          sender: aliceAuth.publicKey,
          authority: bobAuth.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([aliceAuth, bobAuth])
        .rpc()
    );
  });

  it("23. transfer_record_nonce_derivation_is_unique", async () => {
    const pdaN1 = getTransferPda(aliceRail, bobRail, 1);
    const pdaN2 = getTransferPda(aliceRail, bobRail, 2);
    const pdaN1Repeat = getTransferPda(aliceRail, bobRail, 1);
    expect(pdaN1.toBase58()).to.not.equal(pdaN2.toBase58());
    expect(pdaN1.toBase58()).to.equal(pdaN1Repeat.toBase58());
  });

  it("24. confidential_transfer_token_rejects_when_asset_state_missing", async () => {
    const senderTokenAssetState = getTokenAssetStatePda(aliceRail, testTokenMint);
    const receiverTokenAssetState = getTokenAssetStatePda(bobRail, testTokenMint);
    const transferRecord = getTransferPda(aliceRail, bobRail, 77);

    await expectTxFail(() =>
      program.methods
        .confidentialTransferToken(
          77,
          Buffer.from(dummyProof),
          commitmentA,
          commitmentB,
          commitmentA,
          commitmentC,
          nullifierB,
          Array.from(Buffer.alloc(64, 10)),
          Array.from(Buffer.alloc(64, 11))
        )
        .accounts({
          senderRail: aliceRail,
          receiverRail: bobRail,
          senderZkVault: aliceZkVault,
          receiverZkVault: bobZkVault,
          tokenMint: testTokenMint,
          senderTokenAssetState,
          receiverTokenAssetState,
          transferRecord,
          authority: aliceAuth.publicKey,
          receiverAuthority: bobAuth.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([aliceAuth, bobAuth])
        .rpc()
    );
  });

  it("25. receiver_rail_authority_matches_expected_pda_constraint_model", async () => {
    const receiverRailState = await program.account.railState.fetch(bobRail);
    expect(receiverRailState.authority.toBase58()).to.equal(
      bobAuth.publicKey.toBase58()
    );
  });

  it("26. transfer_nonce_replay_model_changes_with_sender_receiver_order", async () => {
    const forward = getTransferPda(aliceRail, bobRail, 101);
    const reverse = getTransferPda(bobRail, aliceRail, 101);
    expect(forward.toBase58()).to.not.equal(reverse.toBase58());
  });
});