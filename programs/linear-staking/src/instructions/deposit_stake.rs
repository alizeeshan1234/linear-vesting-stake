use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, USER_STAKE_SEED, EVENT_AUTHORITY_SEED},
    error::ErrorCode,
    events::StakeDeposited,
    state::{StakeVault, UserStake},
    instructions::helpers::{refresh_user_rewards, update_reward_snapshot_after_stake_change},
    program::LinearStaking,
};

#[derive(Accounts)]
pub struct DepositStake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_account.mint == stake_vault.token_mint,
        constraint = user_token_account.owner == owner.key()
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_TOKEN_ACCOUNT_SEED],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + UserStake::INIT_SPACE,
        seeds = [USER_STAKE_SEED, owner.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    /// CHECK: event authority for emit_cpi
    #[account(seeds = [EVENT_AUTHORITY_SEED], bump)]
    pub event_authority: AccountInfo<'info>,

    pub program: Program<'info, LinearStaking>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositStakeParams {
    pub amount: u64,
}

pub fn handler(ctx: Context<DepositStake>, params: DepositStakeParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;
    let user_stake = &mut ctx.accounts.user_stake;
    let amount = params.amount;
    let clock = Clock::get()?;

    require!(amount > 0, ErrorCode::InvalidAmount);
    
    require!(!stake_vault.is_paused, ErrorCode::VaultPaused);

    if !stake_vault.permissions.allow_deposits {
        return Err(ErrorCode::DepositsNotAllowed.into());
    }

    // Refresh user rewards before changing stake amount
    refresh_user_rewards(user_stake, stake_vault)?;

    // Transfer tokens from user to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.owner.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    transfer(cpi_ctx, amount)?;

    // Update vault stake stats
    stake_vault.stake_stats.total_staked = stake_vault
        .stake_stats
        .total_staked
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    stake_vault.stake_stats.active_amount = stake_vault
        .stake_stats
        .active_amount
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    // Initialize or update user stake
    user_stake.owner = ctx.accounts.owner.key();
    user_stake.is_initialized = true;
    user_stake.stake_vault = stake_vault.key();
    user_stake.bump = ctx.bumps.user_stake;
    user_stake.last_update_timestamp = clock.unix_timestamp;
    user_stake.staked_amount = user_stake
        .staked_amount
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    user_stake.active_stake_amount = user_stake
        .active_stake_amount
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    // Update reward snapshot after stake change
    update_reward_snapshot_after_stake_change(user_stake, stake_vault)?;

    emit_cpi!(StakeDeposited {
        user: ctx.accounts.owner.key(),
        amount,
        total_staked: user_stake.staked_amount,
        active_stake_amount: user_stake.active_stake_amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}