use anchor_lang::prelude::*;

use crate::{
    constants::{STAKE_VAULT_SEED, USER_STAKE_SEED, EVENT_AUTHORITY_SEED},
    error::ErrorCode,
    events::UnstakeCancelled,
    state::{StakeVault, UserStake},
    instructions::helpers::{refresh_user_rewards, update_reward_snapshot_after_stake_change},
    program::LinearStaking,
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

    /// CHECK: event authority for emit_cpi
    #[account(seeds = [EVENT_AUTHORITY_SEED], bump)]
    pub event_authority: AccountInfo<'info>,

    pub program: Program<'info, LinearStaking>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelUnstakeParams {
    pub request_index: u8,
}

pub fn handler(ctx: Context<CancelUnstake>, params: CancelUnstakeParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;
    let user_stake = &mut ctx.accounts.user_stake;
    let current_time = Clock::get()?.unix_timestamp;
    let request_index = params.request_index as usize;

    // Validate request index
    require!(
        request_index < user_stake.unstake_requests.len(),
        ErrorCode::InvalidRequestIndex
    );

    require!(
        !stake_vault.is_paused,
        ErrorCode::VaultPaused
    );

    // Refresh user rewards before changing stake
    refresh_user_rewards(user_stake, stake_vault)?;

    // Get the unstake request
    let unstake_request = &user_stake.unstake_requests[request_index];

    // Calculate remaining unclaimed amount (this is what gets returned to active stake)
    let remaining_amount = unstake_request
        .total_amount
        .checked_sub(unstake_request.claimed_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    require!(remaining_amount > 0, ErrorCode::NoAmountToCancel);

    // Update user stake - move remaining back to active
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

    // Remove the unstake request
    user_stake.unstake_requests.remove(request_index);

    // Update reward snapshot after stake change
    update_reward_snapshot_after_stake_change(user_stake, stake_vault)?;

    user_stake.last_update_timestamp = current_time;

    emit_cpi!(UnstakeCancelled {
        user: ctx.accounts.owner.key(),
        request_index: params.request_index,
        amount_returned: remaining_amount,
        timestamp: current_time,
    });

    Ok(())
}
