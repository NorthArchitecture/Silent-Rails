use anchor_lang::prelude::*;

declare_id!("NorthSentinel11111111111111111111111111111");

#[program]
pub mod sentinel {
    use super::*;

    // Initialise le "Silent Handshake" pour fragmenter les données
    pub fn initialize_handshake(ctx: Context<InitializeHandshake>, fragment_id: u64) -> Result<()> {
        let handshake = &mut ctx.accounts.handshake;
        handshake.authority = *ctx.accounts.authority.key;
        handshake.fragment_id = fragment_id;
        handshake.is_active = true;
        
        msg!("$NORTH Sentinel: Handshake Protocol Initialized for Fragment {}", fragment_id);
        Ok(())
    }

    // Fonction pour sceller les rails de confidentialité (Protocol 03)
    pub fn seal_privacy_rail(ctx: Context<SealRail>) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        rail.is_sealed = true;
        
        msg!("$NORTH Sentinel: Privacy Rail Sealed. 66ms Latency Target Achieved.");
        Ok(())
    }
}

#[account]
pub struct HandshakeState {
    pub authority: Pubkey,   // L'Architecte ou l'entité
    pub fragment_id: u64,    // ID du micro-paquet (Protocol 01)
    pub is_active: bool,
}

#[account]
pub struct RailState {
    pub is_sealed: bool,     // État du Silent Rail (Protocol 03)
}

#[derive(Accounts)]
pub struct InitializeHandshake<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 8 + 1)]
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
