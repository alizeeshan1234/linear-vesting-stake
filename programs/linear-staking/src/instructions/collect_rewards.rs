use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, TRANSFER_AUTHORITY_SEED, USER_STAKE_SEED, EVENT_AUTHORITY_SEED},
    error::ErrorCode,
    events::RewardsCollected,
    state::{StakeVault, UserStake},
    instructions::helpers::refresh_user_rewards,
    program::LinearStaking,
};

#[derive(Accounts)]
pub struct CollectRewards<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [USER_STAKE_SEED, owner.key().as_ref()],
        bump = user_stake.bump,
        constraint = user_stake.owner == owner.key() @ ErrorCode::Unauthorized
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump
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
        bump = stake_vault.token_account_bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    /// CHECK: PDA used as transfer authority
    #[account(
        seeds = [TRANSFER_AUTHORITY_SEED],
        bump = stake_vault.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    /// CHECK: event authority for emit_cpi
    #[account(seeds = [EVENT_AUTHORITY_SEED], bump)]
    pub event_authority: AccountInfo<'info>,

    pub program: Program<'info, LinearStaking>,
}

pub fn handler(ctx: Context<CollectRewards>) -> Result<()> {
    let user_stake = &mut ctx.accounts.user_stake;
    let stake_vault = &mut ctx.accounts.stake_vault;
    let current_time = Clock::get()?.unix_timestamp;

    // Refresh rewards to calculate latest unclaimed amount
    refresh_user_rewards(user_stake, stake_vault)?;

    let rewards_to_claim = user_stake.reward_state.unclaimed_rewards;

    require!(rewards_to_claim > 0, ErrorCode::NoRewardsToClaim);

    // Transfer rewards from vault to user
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

    transfer(cpi_context, rewards_to_claim)?;

    // Update user reward state
    user_stake.reward_state.total_claimed = user_stake
        .reward_state
        .total_claimed
        .checked_add(rewards_to_claim)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.reward_state.unclaimed_rewards = 0;
    user_stake.last_update_timestamp = current_time;

    // Update vault reward state
    stake_vault.reward_state.total_claimed = stake_vault
        .reward_state
        .total_claimed
        .checked_add(rewards_to_claim as u128)
        .ok_or(ErrorCode::MathOverflow)?;

    emit_cpi!(RewardsCollected {
        user: ctx.accounts.owner.key(),
        amount: rewards_to_claim,
        total_claimed: user_stake.reward_state.total_claimed,
        timestamp: current_time,
    });

    Ok(())
}
