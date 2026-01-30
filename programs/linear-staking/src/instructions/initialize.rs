use anchor_lang::prelude::*;
use anchor_spl::{token::{Mint, TokenAccount, Token}};
use crate::{StakeVault, constants::{
    STAKE_VAULT_SEED,
    STAKE_VAULT_TOKEN_ACCOUNT_SEED,
    TRANSFER_AUTHORITY_SEED,
    DEFAULT_VESTING_PERIOD,
    EVENT_AUTHORITY_SEED,
}, StakeStats, RewardState};
use crate::state::stake_vault::StakePermissions;
use crate::events::VaultInitialized;
use crate::program::LinearStaking;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = 8 + StakeVault::INIT_SPACE,
        seeds = [STAKE_VAULT_SEED],
        bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        init,
        payer = admin,
        token::mint = token_mint,
        token::authority = transfer_authority,
        seeds = [STAKE_VAULT_TOKEN_ACCOUNT_SEED],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [TRANSFER_AUTHORITY_SEED],
        bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    /// CHECK: event authority for emit_cpi
    #[account(seeds = [EVENT_AUTHORITY_SEED], bump)]
    pub event_authority: AccountInfo<'info>,

    pub program: Program<'info, LinearStaking>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeParams {
    pub vesting_period: Option<i64>,
}

pub fn handler(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {

    let stake_vault = &mut ctx.accounts.stake_vault;

    stake_vault.is_initialized = true;
    stake_vault.bump = ctx.bumps.stake_vault;
    stake_vault.token_account_bump = ctx.bumps.vault_token_account;
    stake_vault.transfer_authority_bump = ctx.bumps.transfer_authority;
    stake_vault.token_mint = ctx.accounts.token_mint.key();
    stake_vault.vault_token_account = ctx.accounts.vault_token_account.key();
    stake_vault.admin = ctx.accounts.admin.key();
    stake_vault.permissions = StakePermissions {
        allow_deposits: true,
        allow_withdrawals: true,
    };
    stake_vault.vesting_period_seconds = params.vesting_period.unwrap_or(DEFAULT_VESTING_PERIOD) as u64;
    stake_vault.stake_stats = StakeStats::default();
    stake_vault.reward_state = RewardState::default();
    stake_vault.start_time = Clock::get()?.unix_timestamp;
    stake_vault.collective_unstake_requests_count = 0;

    emit_cpi!(VaultInitialized {
        admin: ctx.accounts.admin.key(),
        token_mint: ctx.accounts.token_mint.key(),
        vesting_period_seconds: stake_vault.vesting_period_seconds,
        timestamp: stake_vault.start_time,
    });

    msg!("Stake vault initialized successfully");

    Ok(())
}
