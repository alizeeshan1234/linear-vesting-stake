use anchor_lang::prelude::*;

pub const MAX_UNSTAKE_REQUESTS: usize = 32;

#[account]
#[derive(Debug, InitSpace)]
pub struct UserStake {
    pub owner: Pubkey,
    pub is_initialized: bool,
    pub stake_vault: Pubkey,
    pub staked_amount: u64,
    pub active_stake_amount: u64,
    pub vested_stake_amount: u64,
    #[max_len(MAX_UNSTAKE_REQUESTS)]
    pub unstake_requests: Vec<UnstakeRequest>,
    pub unstake_request_count: u64, //can just use unstake_requests.len() instead
    pub reward_state: UserRewardState,
    pub last_update_timestamp: i64,
    pub bump: u8,
    pub padding: [u8; 8],
}

impl UserStake {
    /// Get the total amount currently unstaking (in linear vesting) across all requests.
    /// This is the remaining unclaimed amount that will gradually become claimable.
    pub fn get_total_unstaking_amount(&self) -> u64 {
        self.unstake_requests
            .iter()
            .map(|req| req.total_amount.saturating_sub(req.claimed_amount))
            .sum()
    }

    /// Get the total claimable amount across all requests
    pub fn get_total_claimable(&self, current_time: i64, vesting_period_seconds: u64) -> u64 {
        self.unstake_requests
            .iter()
            .map(|req| req.claimable_amount(current_time, vesting_period_seconds))
            .sum()
    }

    pub fn cleanup_claimed_requests(&mut self) {
        self.unstake_requests.retain(|req| !req.is_fully_claimed());
    }
}

#[derive(Copy, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Default, Debug, InitSpace)]
pub struct UserRewardState {
    /// User's snapshot of reward_per_token_staked at last update (scaled by PRECISION)
    pub reward_snapshot: u128,
    /// Unclaimed rewards accumulated for this user
    pub unclaimed_rewards: u64,
    /// Total rewards claimed by this user (for tracking/analytics)
    pub total_claimed: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, InitSpace, Default)]
pub struct UnstakeRequest {
    pub total_amount: u64,
    pub claimed_amount: u64,
    pub start_time: i64,
}

impl UnstakeRequest {
    pub fn is_fully_claimed(&self) -> bool {
        self.total_amount > 0 && self.claimed_amount >= self.total_amount
    }

    pub fn claimable_amount(&self, current_time: i64, vesting_period_seconds: u64) -> u64 {
        if current_time <= self.start_time {
            return 0;
        };

        let elapsed_time = (current_time - self.start_time) as u64;
        let end_time = self.start_time + vesting_period_seconds as i64;

        let vested_amount = if current_time >= end_time {
            self.total_amount
        } else {
            self.total_amount
                .checked_mul(elapsed_time)
                .unwrap_or(0)
                .checked_div(vesting_period_seconds)
                .unwrap_or(0)
        };

        // Claimable = vested - already claimed
        vested_amount.saturating_sub(self.claimed_amount)
    }
}
