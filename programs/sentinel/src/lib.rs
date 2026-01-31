use anchor_lang::prelude::*;

declare_id!("zeJyNvmriogt1zPFMMPN6quHjy7YEAXKphsNdpJn11a");

#[program]
pub mod sentinel {
    use super::*;

    pub fn initialize_handshake(
        ctx: Context<InitializeHandshake>, 
        fragment_id: u64,
        zk_evidence: [u8; 32]
    ) -> Result<()> {
        let handshake = &mut ctx.accounts.handshake;
        handshake.authority = *ctx.accounts.authority.key;
        handshake.fragment_id = fragment_id;
        handshake.zk_evidence = zk_evidence;
        handshake.is_active = true;
        msg!("$NORTH Sentinel: Privacy Handshake Initialized.");
        Ok(())
    }

    pub fn seal_privacy_rail(ctx: Context<SealRail>, audit_seal: [u8; 32]) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        rail.audit_seal = audit_seal;
        rail.is_sealed = true;
        msg!("$NORTH Sentinel: Privacy Rail Sealed with Audit Seal.");
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
pub struct InitializeHandshake<'info> {
    // Allocation of 81 bytes to support zk_evidence
    #[account(init, payer = authority, space = 8 + 32 + 8 + 1 + 32)] 
    pub handshake: Account<'info, HandshakeState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SealRail<'info> {
    #[account(mut)]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}
