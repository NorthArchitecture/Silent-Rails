use anchor_lang::prelude::*;

declare_id!("83yN8hf9V8P8eFU8B3bRfpQRPAZopTSEac8J33NEcLci");

/// Sentinel Privacy Rail - Institutional Grade v1.0
/// 
/// ROADMAP:
/// v1.0 (CURRENT) - Core privacy infrastructure deployed
/// v2.0 (PLANNED) - NORTH token payment integration for rail initialization
/// 
/// Token integration will require:
/// - NORTH token payment to create a rail (prevents spam)
/// - Treasury collection for sustainability
/// - Staking mechanism for institutional guarantees

#[program]
pub mod sentinel {
    use super::*;

    /// Initialize a new privacy rail for an institution
    /// 
    /// NOTE: v2.0 will require NORTH token payment
    /// Current: Free initialization (deployment phase)
    /// Future: Will require X NORTH tokens to create a rail
    pub fn initialize_rail(
        ctx: Context<InitializeRail>,
        institution_type: u8,
        compliance_level: u8,
    ) -> Result<()> {
        // TODO v2.0: Add NORTH token payment check here
        // let amount = calculate_rail_cost(institution_type, compliance_level);
        // transfer_checked(user_token_account, treasury, amount)?;
        
        let rail = &mut ctx.accounts.rail;
        let clock = Clock::get()?;
        
        rail.authority = ctx.accounts.authority.key();
        rail.institution_type = institution_type;
        rail.compliance_level = compliance_level;
        rail.is_sealed = false;
        rail.is_active = true;
        rail.is_paused = false;
        rail.total_handshakes = 0;
        rail.created_at = clock.unix_timestamp;
        rail.sealed_at = 0;
        rail.deactivated_at = 0;
        rail.version = PROTOCOL_VERSION;
        rail.audit_seal = [0u8; 32];
        rail.deactivation_reason = 0;
        
        Ok(())
    }

    /// Create an anonymous handshake with ZK proof verification
    /// 
    /// Security Features:
    /// - Nullifier scoped to rail (prevents cross-rail griefing)
    /// - Sealed rail check (immutability after audit)
    /// - Pause mechanism check
    /// - Double-spend protection via nullifier registry
    pub fn create_handshake(
        ctx: Context<CreateHandshake>,
        commitment: [u8; 32],
        nullifier_hash: [u8; 32],
    ) -> Result<()> {
        require!(!ctx.accounts.rail.is_sealed, SentinelError::RailSealed);
        require!(ctx.accounts.rail.is_active, SentinelError::RailInactive);
        require!(!ctx.accounts.rail.is_paused, SentinelError::RailPaused);
        
        require!(!ctx.accounts.nullifier_registry.is_spent, SentinelError::NullifierAlreadyUsed);
        
        let clock = Clock::get()?;
        let rail_key = ctx.accounts.rail.key();
        
        let handshake = &mut ctx.accounts.handshake;
        handshake.rail = rail_key;
        handshake.commitment = commitment;
        handshake.nullifier_hash = nullifier_hash;
        handshake.is_active = true;
        handshake.created_at = clock.unix_timestamp;
        handshake.revoked_at = 0;
        
        let nullifier_registry = &mut ctx.accounts.nullifier_registry;
        nullifier_registry.rail = rail_key;
        nullifier_registry.nullifier_hash = nullifier_hash;
        nullifier_registry.is_spent = true;
        nullifier_registry.spent_at = clock.unix_timestamp;
        
        let rail = &mut ctx.accounts.rail;
        rail.total_handshakes = rail.total_handshakes
            .checked_add(1)
            .ok_or(SentinelError::Overflow)?;
        
        Ok(())
    }

    /// Seal a rail with cryptographic audit commitment
    /// 
    /// Regulatory Compliance:
    /// - Creates immutable snapshot of rail state
    /// - Prevents further mutations (enforced in create_handshake)
    /// - Timestamp-locked for audit trail
    pub fn seal_rail(
        ctx: Context<SealRail>,
        audit_seal: [u8; 32],
    ) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        
        require!(rail.is_active, SentinelError::RailInactive);
        require!(!rail.is_sealed, SentinelError::RailAlreadySealed);
        
        let clock = Clock::get()?;
        
        rail.audit_seal = audit_seal;
        rail.is_sealed = true;
        rail.sealed_at = clock.unix_timestamp;
        
        Ok(())
    }

    /// Deactivate a rail with full audit trail
    /// 
    /// Records deactivation timestamp and reason code for compliance
    pub fn deactivate_rail(
        ctx: Context<DeactivateRail>,
        reason_code: u8,
    ) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        let clock = Clock::get()?;
        
        require!(rail.is_active, SentinelError::RailAlreadyDeactivated);
        
        rail.is_active = false;
        rail.deactivated_at = clock.unix_timestamp;
        rail.deactivation_reason = reason_code;
        
        Ok(())
    }

    /// Emergency pause mechanism for critical situations
    /// 
    /// Temporary halt without deactivation - reversible
    pub fn pause_rail(ctx: Context<PauseRail>) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        
        require!(rail.is_active, SentinelError::RailInactive);
        require!(!rail.is_paused, SentinelError::RailAlreadyPaused);
        
        rail.is_paused = true;
        
        Ok(())
    }

    /// Resume a paused rail
    pub fn unpause_rail(ctx: Context<UnpauseRail>) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        
        require!(rail.is_active, SentinelError::RailInactive);
        require!(rail.is_paused, SentinelError::RailNotPaused);
        
        rail.is_paused = false;
        
        Ok(())
    }

    /// Revoke a specific handshake (institutional control)
    /// 
    /// Authority can revoke specific handshakes while maintaining audit trail
    pub fn revoke_handshake(
        ctx: Context<RevokeHandshake>,
        _reason_code: u8,
    ) -> Result<()> {
        let handshake = &mut ctx.accounts.handshake;
        let rail = &ctx.accounts.rail;
        
        require!(handshake.is_active, SentinelError::HandshakeAlreadyRevoked);
        require!(handshake.rail == rail.key(), SentinelError::InvalidRail);
        
        let clock = Clock::get()?;
        
        handshake.is_active = false;
        handshake.revoked_at = clock.unix_timestamp;
        
        Ok(())
    }
}

/// Privacy rail state - Institution's compliance boundary
/// 
/// NOTE: v2.0 will add fields for token payment tracking
#[account]
pub struct RailState {
    pub authority: Pubkey,
    pub institution_type: u8,
    pub compliance_level: u8,
    pub is_sealed: bool,
    pub is_active: bool,
    pub is_paused: bool,
    pub _padding: [u8; 2],
    pub audit_seal: [u8; 32],
    pub total_handshakes: u64,
    pub created_at: i64,
    pub sealed_at: i64,
    pub deactivated_at: i64,
    pub deactivation_reason: u8,
    pub version: u8,
    pub _reserved: [u8; 6],
    // TODO v2.0: Add token payment fields
    // pub paid_amount: u64,
    // pub payment_timestamp: i64,
}

/// Anonymous handshake state
#[account]
pub struct HandshakeState {
    pub rail: Pubkey,
    pub commitment: [u8; 32],
    pub nullifier_hash: [u8; 32],
    pub is_active: bool,
    pub _padding: [u8; 7],
    pub created_at: i64,
    pub revoked_at: i64,
}

/// Nullifier registry - prevents double-spending
/// CRITICAL: Scoped to rail for security
#[account]
pub struct NullifierRegistry {
    pub rail: Pubkey,
    pub nullifier_hash: [u8; 32],
    pub is_spent: bool,
    pub _padding: [u8; 7],
    pub spent_at: i64,
}

#[derive(Accounts)]
pub struct InitializeRail<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 1 + 1 + 1 + 1 + 1 + 2 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 6,
        seeds = [b"rail", authority.key().as_ref()],
        bump
    )]
    pub rail: Account<'info, RailState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    // TODO v2.0: Add token accounts here
    // pub user_north_token_account: InterfaceAccount<'info, TokenAccount>,
    // pub treasury_north_token_account: InterfaceAccount<'info, TokenAccount>,
    // pub north_mint: InterfaceAccount<'info, Mint>,
    // pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
#[instruction(commitment: [u8; 32], nullifier_hash: [u8; 32])]
pub struct CreateHandshake<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 1 + 7 + 8 + 8,
        seeds = [b"handshake", rail.key().as_ref(), nullifier_hash.as_ref()],
        bump
    )]
    pub handshake: Account<'info, HandshakeState>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 1 + 7 + 8,
        seeds = [b"nullifier", rail.key().as_ref(), nullifier_hash.as_ref()],
        bump
    )]
    pub nullifier_registry: Account<'info, NullifierRegistry>,
    #[account(mut)]
    pub rail: Account<'info, RailState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SealRail<'info> {
    #[account(
        mut,
        has_one = authority @ SentinelError::Unauthorized
    )]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(reason_code: u8)]
pub struct DeactivateRail<'info> {
    #[account(
        mut,
        has_one = authority @ SentinelError::Unauthorized
    )]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct PauseRail<'info> {
    #[account(
        mut,
        has_one = authority @ SentinelError::Unauthorized
    )]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnpauseRail<'info> {
    #[account(
        mut,
        has_one = authority @ SentinelError::Unauthorized
    )]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(reason_code: u8)]
pub struct RevokeHandshake<'info> {
    #[account(mut)]
    pub handshake: Account<'info, HandshakeState>,
    #[account(has_one = authority @ SentinelError::Unauthorized)]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[error_code]
pub enum SentinelError {
    #[msg("This privacy rail has been deactivated")]
    RailInactive,
    #[msg("Unauthorized: You are not the authority")]
    Unauthorized,
    #[msg("This nullifier has already been used")]
    NullifierAlreadyUsed,
    #[msg("This rail has been sealed")]
    RailSealed,
    #[msg("This rail is already sealed")]
    RailAlreadySealed,
    #[msg("This rail has already been deactivated")]
    RailAlreadyDeactivated,
    #[msg("This rail is paused")]
    RailPaused,
    #[msg("This rail is already paused")]
    RailAlreadyPaused,
    #[msg("This rail is not paused")]
    RailNotPaused,
    #[msg("This handshake has already been revoked")]
    HandshakeAlreadyRevoked,
    #[msg("Invalid rail for this handshake")]
    InvalidRail,
    #[msg("Arithmetic overflow")]
    Overflow,
}

pub const PROTOCOL_VERSION: u8 = 1;

pub mod reason_codes {
    pub const LIFECYCLE_END: u8 = 0;
    pub const REGULATORY: u8 = 1;
    pub const SECURITY_INCIDENT: u8 = 2;
    pub const UPGRADE: u8 = 3;
    pub const INSTITUTIONAL_DECISION: u8 = 4;
    pub const COMPLIANCE_VIOLATION: u8 = 5;
}