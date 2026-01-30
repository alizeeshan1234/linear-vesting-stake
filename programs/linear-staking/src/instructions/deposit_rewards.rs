use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED},
    error::ErrorCode,
    state::StakeVault,
};

#[derive(Accounts)]
pub struct DepositRewards<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Admin's token account to deposit rewards from
    #[account(
        mut,
        constraint = admin_token_account.mint == stake_vault.token_mint,
        constraint = admin_token_account.owner == admin.key()
    )]
    pub admin_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump,
        constraint = stake_vault.admin == admin.key() @ ErrorCode::Unauthorized
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_TOKEN_ACCOUNT_SEED],
        bump = stake_vault.token_account_bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositRewardsParams {
    pub amount: u64,
}

pub fn handler(ctx: Context<DepositRewards>, params: DepositRewardsParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;

    // Validate amount
    require!(params.amount > 0, ErrorCode::InvalidAmount);

    // Transfer reward tokens from admin to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.admin_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.admin.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    transfer(cpi_ctx, params.amount)?;

    // Add to pending rewards
    stake_vault.reward_state.pending_rewards = stake_vault
        .reward_state
        .pending_rewards
        .checked_add(params.amount as u128)
        .ok_or(ErrorCode::MathOverflow)?;

    msg!(
        "Deposited {} reward tokens. Total pending rewards: {}",
        params.amount,
        stake_vault.reward_state.pending_rewards
    );

    Ok(())
}
