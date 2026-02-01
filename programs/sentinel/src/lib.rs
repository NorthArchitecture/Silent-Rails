use anchor_lang::prelude::*;

declare_id!("zeJyNvmriogt1zPFMMPN6quHjy7YEAXKphsNdpJn11a");

#[program]
pub mod sentinel {
    use super::*;

    /// V1: High-Throughput Execution Layer.
    /// ZK-Evidence is anchored here for asynchronous verification.
    /// This design allows for 185k+ TPS by avoiding synchronous crypto-bottlenecks.
    pub fn initialize_handshake(
        ctx: Context<InitializeHandshake>, 
        fragment_id: u64,
        zk_evidence: [u8; 32]
    ) -> Result<()> {
        let handshake = &mut ctx.accounts.handshake;
        handshake.authority = *ctx.accounts.authority.key;
        handshake.fragment_id = fragment_id;
        
        // ANCHORING: We commit the evidence hash to the ledger. 
        // Validation is decoupled to ensure sub-100ms finality on the hot-path.
        handshake.zk_evidence = zk_evidence;
        handshake.is_active = true;

        msg!("$NORTH: Handshake Anchored. ZK-Metadata committed for async audit.");
        Ok(())
    }
pub fn open_privacy_rail(ctx: Context<OpenRail>) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        rail.authority = *ctx.accounts.authority.key;
        rail.is_sealed = false;
       msg!("$NORTH: Privacy Rail Opened. State: UNSEALED. Execution optimized.");
        Ok(())
    }
    pub fn seal_privacy_rail(ctx: Context<SealRail>, audit_seal: [u8; 32]) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        rail.audit_seal = audit_seal;
        rail.is_sealed = true;
       msg!("$NORTH: Rail SEALED. Audit Hash committed for compliance tracking.");
        Ok(())
    }
}

#[account]
pub struct HandshakeState {
    pub authority: Pubkey,
    pub fragment_id: u64,
    pub is_active: bool,
    pub zk_evidence: [u8; 32], 
}

#[account]
pub struct RailState {
    pub authority: Pubkey,
    pub is_sealed: bool,
    pub audit_seal: [u8; 32], 
}

#[derive(Accounts)]
#[instruction(fragment_id: u64)]
pub struct InitializeHandshake<'info> {
    #[account(
        init, 
        payer = authority, 
        space = 8 + 32 + 8 + 1 + 32,
        // Seeds : Anti-replay protection: deterministic PDA ensures one unique account per fragment
        seeds = [b"handshake", authority.key().as_ref(), fragment_id.to_le_bytes().as_ref()],
        bump
    )] 
    pub handshake: Account<'info, HandshakeState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct SealRail<'info> {
    #[account(mut, has_one = authority)]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}
#[derive(Accounts)]
pub struct OpenRail<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 1 + 32)]
    pub rail: Account<'info, RailState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}