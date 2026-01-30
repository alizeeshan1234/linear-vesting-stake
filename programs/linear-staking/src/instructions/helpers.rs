use anchor_lang::prelude::*;

use crate::{
    constants::PRECISION,
    error::ErrorCode,
    state::{StakeVault, UserStake},
};

/// Refresh a user's reward state based on current global accumulator.
/// Must be called BEFORE any stake amount changes.
pub fn refresh_user_rewards(
    user_stake: &mut UserStake,
    stake_vault: &StakeVault,
) -> Result<()> {
    let global_reward_per_token = stake_vault.reward_state.reward_per_token_staked;

    if user_stake.active_stake_amount == 0 {
        // No stake, just update snapshot to current global value
        user_stake.reward_state.reward_snapshot = global_reward_per_token;
        return Ok(());
    }

    // Calculate current watermark for user's stake
    let current_watermark = (user_stake.active_stake_amount as u128)
        .checked_mul(global_reward_per_token)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(PRECISION)
        .ok_or(ErrorCode::MathOverflow)?;

    // Calculate pending rewards since last snapshot
    let pending_rewards = current_watermark
        .saturating_sub(user_stake.reward_state.reward_snapshot) as u64;

    // Add to unclaimed rewards
    user_stake.reward_state.unclaimed_rewards = user_stake
        .reward_state
        .unclaimed_rewards
        .checked_add(pending_rewards)
        .ok_or(ErrorCode::MathOverflow)?;

    // Update snapshot to current watermark
    user_stake.reward_state.reward_snapshot = current_watermark;

    Ok(())
}

/// Update user's reward snapshot after stake amount changes.
/// Must be called AFTER stake amount is modified.
pub fn update_reward_snapshot_after_stake_change(
    user_stake: &mut UserStake,
    stake_vault: &StakeVault,
) -> Result<()> {
    let global_reward_per_token = stake_vault.reward_state.reward_per_token_staked;

    // Recalculate watermark with new stake amount
    let new_watermark = (user_stake.active_stake_amount as u128)
        .checked_mul(global_reward_per_token)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(PRECISION)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.reward_state.reward_snapshot = new_watermark;

    Ok(())
}