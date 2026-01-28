use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, USER_STAKE_SEED},
    error::ErrorCode,
    state::{StakeVault, UserStake},
};

#[derive(Accounts)]
pub struct DepositStake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// User's token account to deposit from
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
        init_if_needed,
        payer = fee_payer,
        space = UserStake::LEN,
        seeds = [USER_STAKE_SEED, owner.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    pub token_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositStakeParams {
    pub amount: u64,
}

pub fn handler(ctx: Context<DepositStake>, params: DepositStakeParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;
    let user_stake = &mut ctx.accounts.user_stake;

    // Check permissions
    require!(
        stake_vault.permissions.allow_deposits,
        ErrorCode::DepositsDisabled
    );

    // Validate amount
    require!(params.amount > 0, ErrorCode::InvalidAmount);

    // Initialize user stake if needed
    if !user_stake.is_initialized {
        user_stake.is_initialized = true;
        user_stake.bump = ctx.bumps.user_stake;
        user_stake.owner = ctx.accounts.owner.key();
    }

    // Transfer tokens from user to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.owner.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    transfer(cpi_ctx, params.amount)?;

    // Update user stake
    user_stake.active_stake_amount = user_stake
        .active_stake_amount
        .checked_add(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;

    user_stake.last_update_timestamp = Clock::get()?.unix_timestamp;

    // Update vault stats
    stake_vault.stake_stats.active_amount = stake_vault
        .stake_stats
        .active_amount
        .checked_add(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;

    msg!(
        "Deposited {} tokens. Total active stake: {}",
        params.amount,
        user_stake.active_stake_amount
    );

    Ok(())
}
