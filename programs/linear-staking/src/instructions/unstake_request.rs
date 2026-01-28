use anchor_lang::prelude::*;

use crate::{
    constants::{STAKE_VAULT_SEED, USER_STAKE_SEED},
    error::ErrorCode,
    state::{user_stake::MAX_UNSTAKE_REQUESTS, StakeVault, UnstakeRequest, UserStake},
};

#[derive(Accounts)]
pub struct UnstakeRequestCtx<'info> {
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
pub struct UnstakeRequestParams {
    pub amount: u64,
}

pub fn handler(ctx: Context<UnstakeRequestCtx>, params: UnstakeRequestParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;
    let user_stake = &mut ctx.accounts.user_stake;
    let current_timestamp = Clock::get()?.unix_timestamp;

    // Check permissions
    require!(
        stake_vault.permissions.allow_withdrawals,
        ErrorCode::WithdrawalsDisabled
    );

    // Validate amount
    require!(params.amount > 0, ErrorCode::InvalidAmount);
    require!(
        params.amount <= user_stake.active_stake_amount,
        ErrorCode::InsufficientBalance
    );

    // Check if user has room for another unstake request
    require!(
        (user_stake.unstake_request_count as usize) < MAX_UNSTAKE_REQUESTS,
        ErrorCode::MaxUnstakeRequestsReached
    );

    // Find an empty slot
    let slot_index = user_stake
        .find_empty_slot()
        .ok_or(ErrorCode::MaxUnstakeRequestsReached)?;

    // Create the unstake request with linear vesting
    let end_timestamp = current_timestamp
        .checked_add(stake_vault.vesting_period)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.unstake_requests[slot_index] = UnstakeRequest {
        total_amount: params.amount,
        claimed_amount: 0,
        start_timestamp: current_timestamp,
        end_timestamp,
    };
    user_stake.unstake_request_count = user_stake
        .unstake_request_count
        .checked_add(1)
        .ok_or(ErrorCode::MathOverflow)?;

    // Move tokens from active to pending unlock
    user_stake.active_stake_amount = user_stake
        .active_stake_amount
        .checked_sub(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.last_update_timestamp = current_timestamp;

    // Update vault stats
    stake_vault.stake_stats.active_amount = stake_vault
        .stake_stats
        .active_amount
        .checked_sub(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;

    stake_vault.stake_stats.pending_unlock = stake_vault
        .stake_stats
        .pending_unlock
        .checked_add(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;

    msg!(
        "Unstake request created for {} tokens. Linear unlock over {} seconds (ends at {})",
        params.amount,
        stake_vault.vesting_period,
        end_timestamp
    );

    Ok(())
}
