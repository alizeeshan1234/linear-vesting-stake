use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, USER_STAKE_SEED},
    error::ErrorCode,
    state::{StakeVault, UserStake},
    instructions::helpers::{refresh_user_rewards, update_reward_snapshot_after_stake_change},
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
    pub user_stake: Account<'info, UserS