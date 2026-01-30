use anchor_lang::prelude::*;

use crate::{
    constants::{PRECISION, STAKE_VAULT_SEED, EVENT_AUTHORITY_SEED},
    error::ErrorCode,
    events::RewardsDistributed,
    state::StakeVault,
    program::LinearStaking,
};

#[derive(Accounts)]
pub struct DistributeRewards<'info> {
    /// Anyone can crank this instruction
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    /// CHECK: event authority for emit_cpi
    #[account(seeds = [EVENT_AUTHORITY_SEED], bump)]
    pub event_authority: AccountInfo<'info>,

    pub program: Program<'info, LinearStaking>,
}

pub fn handler(ctx: Context<DistributeRewards>) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;

    let pending = stake_vault.reward_state.pending_rewards;
    let total_active_stake = stake_vault.stake_stats.active_amount;

    // Check if there are rewards to distribute
    require!(pending > 0, ErrorCode::NoPendingRewards);

    // Check if there is active stake to distribute to
    require!(total_active_stake > 0, ErrorCode::NoActiveStake);
    require!(
        !stake_vault.is_paused,
        ErrorCode::VaultPaused
    );

    // Calculate reward per token: (pending * PRECISION) / total_active_stake
    let reward_increment = pending
        .checked_mul(PRECISION)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(total_active_stake as u128)
        .ok_or(ErrorCode::MathOverflow)?;

    // Update global accumulator
    stake_vault.reward_state.reward_per_token_staked = stake_vault
        .reward_state
        .reward_per_token_staked
        .checked_add(reward_increment)
        .ok_or(ErrorCode::MathOverflow)?;

    // Track total distributed
    stake_vault.reward_state.total_distributed = stake_vault
        .reward_state
        .total_distributed
        .checked_add(pending)
        .ok_or(ErrorCode::MathOverflow)?;

    // Clear pending rewards
    stake_vault.reward_state.pending_rewards = 0;

    emit_cpi!(RewardsDistributed {
        distributor: ctx.accounts.payer.key(),
        amount: pending,
        reward_per_token: stake_vault.reward_state.reward_per_token_staked,
        total_active_stake,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
