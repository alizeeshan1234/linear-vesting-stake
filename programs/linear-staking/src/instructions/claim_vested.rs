use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, TRANSFER_AUTHORITY_SEED, USER_STAKE_SEED, EVENT_AUTHORITY_SEED},
    error::ErrorCode,
    events::VestedTokensClaimed,
    state::{StakeVault, UserStake},
    program::LinearStaking,
};

#[derive(Accounts)]
pub struct ClaimVested<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [USER_STAKE_SEED, owner.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        mut,
        constraint = user_token_account.mint == stake_vault.token_mint,
        constraint = user_token_account.owner == owner.key()
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
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

pub fn handler(ctx: Context<ClaimVested>) -> Result<()> {
    let user_stake = &mut ctx.accounts.user_stake;
    let stake_vault = &mut ctx.accounts.stake_vault;
    let current_time = Clock::get()?.unix_timestamp;

    let vesting_period = stake_vault.vesting_period_seconds;
    let mut total_claimable: u64 = 0;

    for unstake_request in user_stake.unstake_requests.iter_mut() {
        let claimable = unstake_request.claimable_amount(current_time, vesting_period);

        if claimable > 0 {
            unstake_request.claimed_amount = unstake_request
                .claimed_amount
                .checked_add(claimable)
                .ok_or(ErrorCode::MathOverflow)?;
            total_claimable = total_claimable
                .checked_add(claimable)
                .ok_or(ErrorCode::MathOverflow)?;
        }
    }

    require!(total_claimable > 0, ErrorCode::NoClaimableAmount);

    // Transfer tokens from vault to user
    let authority_seeds: &[&[&[u8]]] = &[&[
        TRANSFER_AUTHORITY_SEED,
        &[stake_vault.transfer_authority_bump],
    ]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.transfer_authority.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_context = CpiContext::new_with_signer(cpi_program, cpi_accounts, authority_seeds);

    transfer(cpi_context, total_claimable)?;

    // Update vault stats
    stake_vault.stake_stats.unstaking_amount = stake_vault
        .stake_stats
        .unstaking_amount
        .checked_sub(total_claimable)
        .ok_or(ErrorCode::MathOverflow)?;

    stake_vault.stake_stats.total_staked = stake_vault
        .stake_stats
        .total_staked
        .checked_sub(total_claimable)
        .ok_or(ErrorCode::MathOverflow)?;

    stake_vault.stake_stats.total_vested = stake_vault
        .stake_stats
        .total_vested
        .checked_add(total_claimable)
        .ok_or(ErrorCode::MathOverflow)?;

    // Update user stake
    user_stake.vested_stake_amount = user_stake
        .vested_stake_amount
        .checked_add(total_claimable)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.staked_amount = user_stake
        .staked_amount
        .checked_sub(total_claimable)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.cleanup_claimed_requests();
    user_stake.last_update_timestamp = current_time;

    emit_cpi!(VestedTokensClaimed {
        user: ctx.accounts.owner.key(),
        amount: total_claimable,
        remaining_unstaking: user_stake.get_total_unstaking_amount(),
        timestamp: current_time,
    });

    Ok(())
}