use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    constants::{DEFAULT_VESTING_PERIOD, STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, TRANSFER_AUTHORITY_SEED},
    state::{StakePermissions, StakeVault},
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// The token mint for the token being staked
    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = StakeVault::LEN,
        seeds = [STAKE_VAULT_SEED],
        bump
    )]
    pub stake_vault: Account<'info, StakeVault>,

    #[account(
        init,
        payer = admin,
        seeds = [STAKE_VAULT_TOKEN_ACCOUNT_SEED],
        bump,
        token::mint = token_mint,
        token::authority = transfer_authority,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    /// CHECK: PDA used as transfer authority
    #[account(
        seeds = [TRANSFER_AUTHORITY_SEED],
        bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeParams {
    /// Optional custom vesting period in seconds (defaults to 30 days)
    pub vesting_period: Option<i64>,
}

pub fn handler(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;

    stake_vault.is_initialized = true;
    stake_vault.bump = ctx.bumps.stake_vault;
    stake_vault.token_account_bump = ctx.bumps.vault_token_account;
    stake_vault.transfer_authority_bump = ctx.bumps.transfer_authority;
    stake_vault.token_mint = ctx.accounts.token_mint.key();
    stake_vault.vault_token_account = ctx.accounts.vault_token_account.key();
    stake_vault.admin = ctx.accounts.admin.key();
    stake_vault.permissions = StakePermissions {
        allow_deposits: true,
        allow_withdrawals: true,
    };
    stake_vault.vesting_period = params.vesting_period.unwrap_or(DEFAULT_VESTING_PERIOD);

    msg!("Stake vault initialized with vesting period: {} seconds", stake_vault.vesting_period);

    Ok(())
}
