use anchor_lang::prelude::*;
use anchor_spl::token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, TRANSFER_AUTHORITY_SEED, USER_STAKE_SEED},
    error::ErrorCode,
    state::{StakeVault, UserStake},
};

#[derive(Accounts)]
pub struct ClaimVested<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// User's token account to receive claimed tokens
    #[account(
        mut,
        constraint = user_token_account.mint == stake_vault.token_mint,
        constraint = user_token_account.owner == owner.key()
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

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
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: PDA used as transfer authority
    #[account(
        seeds = [TRANSFER_AUTHORITY_SEED],
        bump = stake_vault.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [USER_STAKE_SEED, owner.key().as_ref()],
        bump = user_stake.bump,
        constraint = user_stake.owner == owner.key() @ ErrorCode::Unauthorized
    )]
    pub user_stake: Account<'info, UserStake>,

    pub token_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ClaimVestedParams {
    /// Optional: specific request ID to claim from. If None, claims from all requests.
    pub request_id: Option<u8>,
}

pub fn handler(ctx: Context<ClaimVested>, params: ClaimVestedParams) -> Result<()> {
    let stake_vault = &ctx.accounts.stake_vault;
    let user_stake = &mut ctx.accounts.user_stake;
    let current_timestamp = Clock::get()?.unix_timestamp;

    // Check permissions
    require!(
        stake_vault.permissions.allow_withdrawals,
        ErrorCode::WithdrawalsDisabled
    );

    let total_claimable: u64;

    if let Some(request_id) = params.request_id {
        // Claim from specific request
        let idx = request_id as usize;
        require!(
            idx < user_stake.unstake_request_count as usize,
            ErrorCode::InvalidUnstakeRequestId
        );

        let request = &mut user_stake.unstake_requests[idx];
        let claimable = request.get_claimable_amount(current_timestamp);
        require!(claimable > 0, ErrorCode::NoVestedTokens);

        request.claimed_amount = request
            .claimed_amount
            .checked_add(claimable)
            .ok_or(ErrorCode::MathOverflow)?;

        total_claimable = claimable;
    } else {
        // Claim from all requests
        total_claimable = user_stake.get_total_claimable(current_timestamp);
        require!(total_claimable > 0, ErrorCode::NoVestedTokens);

        // Update claimed amounts for each request
        for request in user_stake.unstake_requests.iter_mut() {
            if !request.is_empty() {
                let claimable = request.get_claimable_amount(current_timestamp);
                if claimable > 0 {
                    request.claimed_amount = request
                        .claimed_amount
                        .checked_add(claimable)
                        .ok_or(ErrorCode::MathOverflow)?;
                }
            }
        }
    }

    // Transfer tokens from vault to user
    let authority_seeds: &[&[&[u8]]] = &[&[
        TRANSFER_AUTHORITY_SEED,
        &[stake_vault.transfer_authority_bump],
    ]];

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.vault_token_account.to_account_info(),
        mint: ctx.accounts.token_mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.transfer_authority.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, authority_seeds);
    transfer_checked(cpi_ctx, total_claimable, ctx.accounts.token_mint.decimals)?;

    // Update vault stats
    let stake_vault = &mut ctx.accounts.stake_vault;
    stake_vault.stake_stats.pending_unlock = stake_vault
        .stake_stats
        .pending_unlock
        .checked_sub(total_claimable)
        .ok_or(ErrorCode::MathOverflow)?;

    // Clean up fully claimed requests
    user_stake.cleanup_claimed_requests();
    user_stake.last_update_timestamp = current_timestamp;

    msg!(
        "Claimed {} vested tokens. Remaining pending unlock: {}",
        total_claimable,
        user_stake.get_total_pending_unlock()
    );

    Ok(())
}
