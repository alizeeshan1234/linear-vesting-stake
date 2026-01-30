use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    constants::{STAKE_VAULT_SEED, STAKE_VAULT_TOKEN_ACCOUNT_SEED, TRANSFER_AUTHORITY_SEED, EVENT_AUTHORITY_SEED},
    error::ErrorCode,
    program::LinearStaking,
    StakeVault,
};

#[derive(Accounts)]
pub struct PauseVault<'info> {
    #[account(
        mut,
        constraint = admin.key() == stake_vault.admin @ ErrorCode::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump
    )]
    pub stake_vault: Account<'info, StakeVault>,
}

pub fn pause_handler(ctx: Context<PauseVault>) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;
    
    require!(
        !stake_vault.is_paused,
        ErrorCode::VaultAlreadyPaused
    );

    stake_vault.is_paused = true;
    msg!("Stake vault has been paused");
    Ok(())
}

#[derive(Accounts)]
pub struct UnpauseVault<'info> {
    #[account(
        mut,
        constraint = admin.key() == stake_vault.admin @ ErrorCode::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump
    )]
    pub stake_vault: Account<'info, StakeVault>,
}

pub fn unpause_handler(ctx: Context<UnpauseVault>) -> Result<()> {                                                                                                                                                 
    let stake_vault = &mut ctx.accounts.stake_vault;  

    require!(
        stake_vault.is_paused, ErrorCode::NotPaused
    );

    stake_vault.is_paused = false;                                                                                                                                                                                                                                                                                                                                                                                                 
    msg!("Vault unpaused");                                                                                                                                                                                        
    Ok(())                                                                                                                                                                                                         
}

#[derive(Accounts)]
pub struct UpdateVestingPeriod<'info> {
    #[account(
        mut,
        constraint = admin.key() == stake_vault.admin @ ErrorCode::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump
    )]
    pub stake_vault: Account<'info, StakeVault>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateVestingPeriodParams {
    pub new_vesting_period_seconds: u64,
}

pub fn update_vesting_period_handler(
    ctx: Context<UpdateVestingPeriod>,
    params: UpdateVestingPeriodParams,
) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;

    require!(params.new_vesting_period_seconds > 0, ErrorCode::InvalidVestingPeriod);

    stake_vault.vesting_period_seconds = params.new_vesting_period_seconds;

    msg!(
        "Vesting period updated to {} seconds",
        stake_vault.vesting_period_seconds
    );
    Ok(())
}

#[derive(Accounts)]                                                                                                                                                                                                
pub struct EmergencyWithdrawCtx<'info> {                                                                                                                                                                           
    #[account(                                                                                                                                                                                                     
        mut,                                                                                                                                                                                                       
        constraint = admin.key() == stake_vault.admin @ ErrorCode::Unauthorized                                                                                                                                    
    )]                                                                                                                                                                                                             
    pub admin: Signer<'info>,                                                                                                                                                                                      
                                                                                                                                                                                                                    
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
        constraint = admin_token_account.owner == admin.key() @ ErrorCode::Unauthorized                                                                                                                            
    )]                                                                                                                                                                                                             
    pub admin_token_account: Account<'info, TokenAccount>,                                                                                                                                                         
                                                                                                                                                                                                                    
    /// CHECK: PDA authority for token transfers                                                                                                                                                                   
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
                                                                                                                                                                                                                    
#[derive(AnchorSerialize, AnchorDeserialize)]                                                                                                                                                                      
pub struct EmergencyWithdrawParams {                                                                                                                                                                               
    pub amount: u64,                                                                                                                                                                                               
}     

pub fn emergency_withdraw_handler(
    ctx: Context<EmergencyWithdrawCtx>,
    params: EmergencyWithdrawParams,
) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;

    // Must be paused for emergency withdraw
    require!(stake_vault.is_paused, ErrorCode::VaultNotPaused);

    let vault_balance = ctx.accounts.vault_token_account.amount;
    let withdraw_amount = if params.amount == 0 {
        vault_balance
    } else {
        params.amount
    };

    require!(withdraw_amount > 0, ErrorCode::InvalidAmount);
    require!(withdraw_amount <= vault_balance, ErrorCode::InsufficientVaultBalance);

    // Transfer tokens from vault to admin
    let seeds = &[TRANSFER_AUTHORITY_SEED, &[stake_vault.transfer_authority_bump]];
    let signer_seeds = &[&seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.admin_token_account.to_account_info(),
                authority: ctx.accounts.transfer_authority.to_account_info(),
            },
            signer_seeds,
        ),
        withdraw_amount,
    )?;

    msg!("Emergency withdraw: {} tokens", withdraw_amount);
    Ok(())
}

// ========================================================================
// Permission Management
// ========================================================================

#[derive(Accounts)]
pub struct UpdatePermissions<'info> {
    #[account(
        constraint = admin.key() == stake_vault.admin @ ErrorCode::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [STAKE_VAULT_SEED],
        bump = stake_vault.bump
    )]
    pub stake_vault: Account<'info, StakeVault>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdatePermissionsParams {
    pub allow_deposits: Option<bool>,
    pub allow_withdrawals: Option<bool>,
}

pub fn update_permissions_handler(
    ctx: Context<UpdatePermissions>,
    params: UpdatePermissionsParams,
) -> Result<()> {
    let stake_vault = &mut ctx.accounts.stake_vault;

    if let Some(allow_deposits) = params.allow_deposits {
        stake_vault.permissions.allow_deposits = allow_deposits;
        msg!("Deposits permission set to: {}", allow_deposits);
    }

    if let Some(allow_withdrawals) = params.allow_withdrawals {
        stake_vault.permissions.allow_withdrawals = allow_withdrawals;
        msg!("Withdrawals permission set to: {}", allow_withdrawals);
    }

    Ok(())
}         