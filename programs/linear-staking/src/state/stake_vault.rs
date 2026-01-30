use anchor_lang::prelude::*;

pub const MAX_UNSTAKE_REQUESTS: usize = 32;

#[account]
#[derive(Debug, InitSpace)]
pub struct StakeVault {
    pub is_initialized: bool,
    pub bump: u8,
    pub token_account_bump: u8,
    pub transfer_authority_bump: u8,
    pub token_mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub admin: Pubkey,
    pub permissions: StakePermissions,
    pub vesting_period_seconds: u64, // no end time
    pub stake_stats: StakeStats,
    pub reward_state: RewardState,
    pub start_time: i64,
    pub collective_unstake_requests_count: u64,
    pub padding: [u8; 8],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, InitSpace, Default)]
pub struct StakePermissions {
    pub allow_deposits: bool,
    pub allow_withdrawals: bool,
}

#[account]
#[derive(Debug, InitSpace, Default)]
pub struct StakeStats {
    pub total_staked: u64,      // total tokens in vault (active + unstaking) decreases on claims for linear vested tokens
    pub active_amount: u64,     // total tokens staked currently (earning rewards)
    pub unstaking_amount: u64,  // total tokens in linear vesting (not earning rewards)
    pub total_vested: u64,      // total tokens claimed from linear vesting (cumulative)
}

#[account]
#[derive(Debug, InitSpace, Default)]
pub struct RewardState {
    /// Rewards deposited but not yet distributed to the accumulator
    pub pending_rewards: u128,
    /// Cumulative rewards per token staked (scaled by PRECISION)
    pub reward_per_token_staked: u128,
    /// Total rewards that have been distributed to the accumulator
    pub total_distributed: u128,
    /// Total rewards that have been claimed by users
    pub total_claimed: u128,
}