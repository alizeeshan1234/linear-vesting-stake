use anchor_lang::prelude::*;

#[event]
pub struct VaultInitialized {
    pub admin: Pubkey,
    pub token_mint: Pubkey,
    pub vesting_period_seconds: u64,
    pub timestamp: i64,
}

#[event]
pub struct StakeDeposited {
    pub user: Pubkey,
    pub amount: u64,
    pub total_staked: u64,
    pub active_stake_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct UnstakeRequested {
    pub user: Pubkey,
    pub amount: u64,
    pub request_index: u64,
    pub start_time: i64,
    pub end_time: i64,
}

#[event]
pub struct VestedTokensClaimed {
    pub user: Pubkey,
    pub amount: u64,
    pub remaining_unstaking: u64,
    pub timestamp: i64,
}

#[event]
pub struct UnstakeCancelled {
    pub user: Pubkey,
    pub request_index: u8,
    pub amount_returned: u64,
    pub timestamp: i64,
}

#[event]
pub struct RewardsDeposited {
    pub admin: Pubkey,
    pub amount: u64,
    pub total_pending: u128,
    pub timestamp: i64,
}

#[event]
pub struct RewardsDistributed {
    pub distributor: Pubkey,
    pub amount: u128,
    pub reward_per_token: u128,
    pub total_active_stake: u64,
    pub timestamp: i64,
}

#[event]
pub struct RewardsCollected {
    pub user: Pubkey,
    pub amount: u64,
    pub total_claimed: u64,
    pub timestamp: i64,
}