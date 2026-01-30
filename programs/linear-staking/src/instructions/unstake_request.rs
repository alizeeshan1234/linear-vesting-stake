use anchor_lang::prelude::*;

use crate::{
    constants::{STAKE_VAULT_SEED, USER_STAKE_SEED, EVENT_AUTHORITY_SEED},
    error::ErrorCode,
    events::UnstakeRequested,
    state::{user_stake::MAX_UNSTAKE_REQUESTS, StakeVault, UnstakeRequest, UserStake},
    instructions::helpers::{refresh_user_rewards, update_reward_snapshot_after_stake_change},
    program::LinearStaking,
};

#[derive(Accounts)]
pub struct UnstakeRequestCtx<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        mut,
        seeds = [USER_STAKE_SEED, owner.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    /// CHECK: event authority for emit_cpi
    #[account(seeds = [EVENT_AUTHORITY_SEED], bump)]
    pub event_authority: AccountInfo<'info>,

    pub program: Program<'info, LinearStaking>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UnstakeRequestParams {
    pub amount: u64,
}

pub fn handler(ctx: Context<UnstakeRequestCtx>, params: UnstakeRequestParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;
    let user_stake = &mut ctx.accounts.user_stake;
    let current_time = Clock::get()?.unix_timestamp;

    require!(
        stake_vault.permissions.allow_withdrawals,
        ErrorCode::WithdrawalsDisabled
    );

    require!(
        params.amount > 0 && params.amount <= user_stake.active_stake_amount,
        ErrorCode::InvalidAmount
    );

    require!(
        user_stake.unstake_requests.len() < MAX_UNSTAKE_REQUESTS,
        ErrorCode::MaxUnstakeRequestsReached
    );

    // Refresh user rewards before changing stake
    refresh_user_rewards(user_stake, stake_vault)?;

    // Create new unstake request
    user_stake.unstake_requests.push(UnstakeRequest {
        total_amount: params.amount,
        claimed_amount: 0,
        start_time: current_time,
    });

    // Update user stake amounts
    user_stake.active_stake_amount = user_stake
        .active_stake_amount
        .checked_sub(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.last_update_timestamp = current_time;

    // Update vault stake stats
    stake_vault.stake_stats.active_amount = stake_vault
        .stake_stats
        .active_amount
        .checked_sub(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;

    stake_vault.stake_stats.unstaking_amount = stake_vault
        .stake_stats
        .unstaking_amount
        .checked_add(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;

    stake_vault.collective_unstake_requests_count = stake_vault
        .collective_unstake_requests_count
        .checked_add(1)
        .ok_or(ErrorCode::MathOverflow)?;

    // Update reward snapshot after stake change
    update_reward_snapshot_after_stake_change(user_stake, stake_vault)?;

    let end_time = current_time + stake_vault.vesting_period_seconds as i64;

    emit_cpi!(UnstakeRequested {
        user: ctx.accounts.owner.key(),
        amount: params.amount,
        request_index: user_stake.unstake_requests.len() as u64 - 1,
        start_time: current_time,
        end_time,
    });

    Ok(())
}
