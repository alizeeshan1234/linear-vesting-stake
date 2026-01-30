use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, TRANSFER_AUTHORITY_SEED, USER_STAKE_SEED},
    error::ErrorCode,
    state::{StakeVault, UserStake},
    instructions::helpers::refresh_user_rewards,
};

#[derive(Accounts)]
pub struct CollectRewards<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// User's token account to receive rewards
    #[account(
        mut,
        constraint = user_token_account.mint == stake_vault.token_mint,
        constraint = user_token_account.owner == owner.key()
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_TOKEN_ACCOUNT_SEED],
        bump = stake_vault.token_account_bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [USER_STAKE_SEED, owner.key().as_ref()],
        bump = user_stake.bump,
        constraint = user_stake.owner == owner.key() @ ErrorCode::Unauthorized
    )]
    pub user_stake: Account<'info, UserStake>,

    /// CHECK: PDA used as transfer authority
    #[account(
        seeds = [TRANSFER_AUTHORITY_SEED],
        bump = stake_vault.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CollectRewardsParams {}

pub fn handler(ctx: Context<CollectRewards>, _params: CollectRewardsParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;
    let user_stake = &mut ctx.accounts.user_stake;

    // Refresh user rewards to get latest accumulated amount
    refresh_user_rewards(user_stake, stake_vault)?;

    let reward_amount = user_stake.reward_state.unclaimed_rewards;
    require!(reward_amount > 0, ErrorCode::NoRewardsToClaim);

    // Transfer rewards to user
    let authority_seeds: &[&[&[u8]]] = &[&[
        TRANSFER_AUTHORITY_SEED,
        &[stake_vault.transfer_authority_bump],
    ]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.transfer_authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        authority_seeds,
    );
    transfer(cpi_ctx, reward_amount)?;

    // Update user state
    user_stake.reward_state.unclaimed_rewards = 0;
    user_stake.reward_state.total_claimed = user_stake
        .reward_state
        .total_claimed
        .checked_add(reward_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    // Update vault state
    stake_vault.reward_state.total_claimed = stake_vault
        .reward_state
        .total_claimed
        .checked_add(reward_amount as u128)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.last_update_timestamp = Clock::get()?.unix_timestamp;

    msg!(
        "Collected {} reward tokens. Total claimed by user: {}",
        reward_amount,
        user_stake.reward_state.total_claimed
    );

    Ok(())
}
