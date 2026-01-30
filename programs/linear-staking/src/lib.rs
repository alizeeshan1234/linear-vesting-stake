#![allow(ambiguous_glob_reexports)]

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("8EipwhySUKEhFULikiYurRNZezFrE3ttpJELCTQDCbgx");

#[program]
pub mod linear_staking {
    use super::*;

    /// Initialize the stake vault with a token mint and optional custom vesting period
    pub fn initialize(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
        initialize::handler(ctx, params) 
    }

    /// Deposit tokens into the staking vault
    pub fn deposit_stake(ctx: Context<DepositStake>, params: DepositStakeParams) -> Result<()> {
        deposit_stake::handler(ctx, params)
    }

    /// Request to unstake tokens - starts the linear vesting period
    pub fn unstake_request(ctx: Context<UnstakeRequestCtx>, params: UnstakeRequestParams) -> Result<()> {
        unstake_request::handler(ctx, params)
    }

    /// Claim vested (unlocked) tokens from unstake requests
    pub fn claim_vested(ctx: Context<ClaimVested>, params: ClaimVestedParams) -> Result<()> {
        claim_vested::handler(ctx, params)
    }

    /// Cancel an unstake request and return remaining tokens to active stake
    pub fn cancel_unstake(ctx: Context<CancelUnstake>, params: CancelUnstakeParams) -> Result<()> {
        cancel_unstake::handler(ctx, params)
    }

    /// Admin deposits reward tokens into the vault
    pub fn deposit_rewards(ctx: Context<DepositRewards>, params: DepositRewardsParams) -> Result<()> {
        deposit_rewards::handler(ctx, params)
    }

    /// Distribute pending rewards to the global accumulator (permissionless crank)
    pub fn distribute_rewards(ctx: Context<DistributeRewards>, params: DistributeRewardsParams) -> Result<()> {
        distribute_rewards::handler(ctx, params)
    }

    /// User collects their accumulated rewards
    pub fn collect_rewards(ctx: Context<CollectRewards>, params: CollectRewardsParams) -> Result<()> {
        collect_rewards::handler(ctx, params)
    }
}
