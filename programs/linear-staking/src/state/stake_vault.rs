use anchor_lang::prelude::*;

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct StakePermissions {
    pub allow_deposits: bool,
    pub allow_withdrawals: bool,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct StakeStats {
    pub total_staked: u64,      // total tokens in vault (active + unstaking) decreases on claims for linear vested tokens
    pub active_amount: u64,     // total tokens staked currently (earning rewards)
    pub unstaking_amount: u64,  // total tokens in linear vesting (not earning rewards)
    pub total_vested: u64,      // total tokens claimed from linear vesting (cumulative)
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
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

#[account]
#[derive(Default, Debug)]
pub struct StakeVault {
    /// Whether the vault is initialized
    pub is_initialized: bool,
    /// Bump seed for PDA
    pub bump: u8,
    /// Bump seed for token account PDA
    pub token_account_bump: u8,
    /// Bump seed for transfer authority PDA
    pub transfer_authority_bump: u8,
    /// The mint of the token being staked (also used for rewards)
    pub token_mint: Pubkey,
    /// The vault's token account (holds both staked tokens and rewards)
    pub vault_token_account: Pubkey,
    /// Admin authority that can update vault settings
    pub admin: Pubkey,
    /// Permissions for deposits/withdrawals
    pub permissions: StakePermissions,
    /// Linear vesting period in seconds (default 30 days = 2,592,000 seconds)
    pub vesting_period: i64,
    /// Global stake statistics
    pub stake_stats: StakeStats,
    /// Reward distribution state
    pub reward_state: RewardState,
    /// Padding for future use
    pub padding: [u64; 4],
}

impl StakeVault {
    pub const LEN: usize = 8 + std::mem::size_of::<StakeVault>();
}
