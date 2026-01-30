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

    // Calculate current watermark for user's stake
    // watermark = stake * global_rate / PRECISION
    let current_watermark = (user_stake.active_stake_amount as u128)
        .checked_mul(global_reward_per_token)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(PRECISION)
        .ok_or(ErrorCode::MathOverflow)?;

    if user_stake.active_stake_amount == 0 {
        // No stake, set snapshot to 0 (correct watermark for 0 stake)
        // When user deposits, update_reward_snapshot_after_stake_change must be called
        user_stake.reward_state.reward_snapshot = current_watermark;
        return Ok(());
    }

    // Calculate pending rewards since last snapshot
    // Both current_watermark and reward_snapshot are in the same units (watermark)
    let pending_rewards = current_watermark
        .saturating_sub(user_stake.reward_state.reward_snapshot) as u64;

    // Add to unclaimed rewards
    user_stake.reward_state.unclaimed_rewards = user_stake
        .reward_state
        .unclaimed_rewards
        .checked_add(pending_rewards)
        .ok_or(ErrorCode::MathOverflow)?;

    // Update snapshot to current watermark (not raw global_reward_per_token)
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