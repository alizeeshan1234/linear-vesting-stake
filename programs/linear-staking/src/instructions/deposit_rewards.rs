use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, EVENT_AUTHORITY_SEED},
    error::ErrorCode,
    events::RewardsDeposited,
    state::StakeVault,
    program::LinearStaking,
};

#[derive(Accounts)]
pub struct DepositRewards<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = admin_token_account.mint == stake_vault.token_mint,
        constraint = admin_token_account.owner == admin.key()
    )]
    pub admin_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_TOKEN_ACCOUNT_SEED],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

    /// CHECK: event authority for emit_cpi
    #[account(seeds = [EVENT_AUTHORITY_SEED], bump)]
    pub event_authority: AccountInfo<'info>,

    pub program: Program<'info, LinearStaking>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositRewardsParams {
    pub amount: u64,
}

pub fn handler(ctx: Context<DepositRewards>, params: DepositRewardsParams) -> Result<()> {

    let stake_vault = &mut ctx.accounts.stake_vault;
    let amount = params.amount;

    require!(
        ctx.accounts.admin.key() == stake_vault.admin,
        ErrorCode::Unauthorized
    );
    require!(amount > 0, ErrorCode::InvalidAmount);

    let cpi_accounts = Transfer {
        from: ctx.accounts.admin_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.admin.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    transfer(cpi_ctx, amount)?;

    stake_vault.reward_state.pending_rewards = stake_vault
        .reward_state
        .pending_rewards
        .checked_add(amount as u128)
        .ok_or(ErrorCode::MathOverflow)?;

    emit_cpi!(RewardsDeposited {
        admin: ctx.accounts.admin.key(),
        amount,
        total_pending: stake_vault.reward_state.pending_rewards,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}