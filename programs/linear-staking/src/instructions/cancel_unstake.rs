use anchor_lang::prelude::*;

use crate::{
    constants::{STAKE_VAULT_SEED, USER_STAKE_SEED},
    error::ErrorCode,
    state::{StakeVault, UnstakeRequest, UserStake},
    instructions::helpers::{refresh_user_rewards, update_reward_snapshot_after_stake_change},
};

#[derive(Accounts)]
pub struct CancelUnstake<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        mut,
        seeds = [USER_STAKE_SEED, owner.key().as_ref()],
        bump = user_stake.bump,
        constraint = user_stake.owner == owner.key() @ ErrorCode::Unauthorized
    )]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelUnstakeParams {
    /// The request ID to cancel
    pub request_id: u8,
}

pub fn handler(ctx: Context<CancelUnstake>, params: CancelUnstakeParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;
    let user_stake = &mut ctx.accounts.user_stake;
    let current_timestamp = Clock::get()?.unix_timestamp;

    let idx = params.request_id as usize;
    require!(
        idx < user_stake.unstake_request_count as usize,
        ErrorCode::InvalidUnstakeRequestId
    );

    // Validate request exists and capture data before mutable borrow
    require!(
        !user_stake.unstake_requests[idx].is_empty(),
        ErrorCode::InvalidUnstakeRequestId
    );

    // IMPORTANT: Refresh rewards BEFORE changing stake amount
    refresh_user_rewards(user_stake, stake_vault)?;

    // Calculate the remaining unclaimed amount (what we're canceling)
    // The user keeps what they've already claimed, but the rest goes back to active stake
    let remaining_amount = user_stake.unstake_requests[idx]
        .total_amount
        .checked_sub(user_stake.unstake_requests[idx].claimed_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    // Move remaining tokens back to active stake
    user_stake.active_stake_amount = user_stake
        .active_stake_amount
        .checked_add(remaining_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    // Update vault stats
    stake_vault.stake_stats.active_amount = stake_vault
        .stake_stats
        .active_amount
        .checked_add(remaining_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    stake_vault.stake_stats.unstaking_amount = stake_vault
        .stake_stats
        .unstaking_amount
        .checked_sub(remaining_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    // Clear the request by swapping with the last one
    let last_idx = (user_stake.unstake_request_count - 1) as usize;
    if idx != last_idx {
        user_stake.unstake_requests.swap(idx, last_idx);
    }
    user_stake.unstake_requests[last_idx] = UnstakeRequest::default();
    user_stake.unstake_request_count -= 1;

    // Update reward snapshot for new stake amount
    update_reward_snapshot_after_stake_change(user_stake, stake_vault)?;

    user_stake.last_update_timestamp = current_timestamp;

    msg!(
        "Cancelled unstake request. {} tokens returned to active stake. Active stake: {}",
        remaining_amount,
        user_stake.active_stake_amount
    );

    Ok(())
}
