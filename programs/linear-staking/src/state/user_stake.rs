use anchor_lang::prelude::*;

/// Represents a single unstake request with linear vesting
#[derive(Copy, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct UnstakeRequest {
    /// Total amount requested for unstaking
    pub total_amount: u64,
    /// Amount already claimed from this request
    pub claimed_amount: u64,
    /// Timestamp when the unstake request was initiated
    pub start_timestamp: i64,
    /// Timestamp when the vesting period ends (start + vesting_period)
    pub end_timestamp: i64,
}

impl UnstakeRequest {
    /// Calculate the amount that has vested (unlocked) based on current time
    /// Linear vesting: amount unlocks proportionally over the vesting period
    pub fn get_vested_amount(&self, current_timestamp: i64) -> u64 {
        if self.total_amount == 0 {
            return 0;
        }

        if current_timestamp >= self.end_timestamp {
            // Fully vested
            return self.total_amount;
        }

        if current_timestamp <= self.start_timestamp {
            // Nothing vested yet
            return 0;
        }

        // Linear interpolation: vested = total * (elapsed / duration)
        let elapsed = (current_timestamp - self.start_timestamp) as u128;
        let duration = (self.end_timestamp - self.start_timestamp) as u128;

        let vested = (self.total_amount as u128)
            .checked_mul(elapsed)
            .and_then(|v| v.checked_div(duration))
            .unwrap_or(0) as u64;

        vested
    }

    /// Calculate the claimable amount (vested but not yet claimed)
    pub fn get_claimable_amount(&self, current_timestamp: i64) -> u64 {
        let vested = self.get_vested_amount(current_timestamp);
        vested.saturating_sub(self.claimed_amount)
    }

    /// Check if this request slot is empty/unused
    pub fn is_empty(&self) -> bool {
        self.total_amount == 0
    }

    /// Check if this request is fully claimed
    pub fn is_fully_claimed(&self) -> bool {
        self.total_amount > 0 && self.claimed_amount >= self.total_amount
    }
}

/// Maximum number of concurrent unstake requests per user
pub const MAX_UNSTAKE_REQUESTS: usize = 5;

#[account]
#[derive(Default, Debug)]
pub struct UserStake {
    /// Owner of this stake account
    pub owner: Pubkey,
    /// Whether the account is initialized
    pub is_initialized: bool,
    /// Bump seed for PDA
    pub bump: u8,
    /// Number of active unstake requests
    pub unstake_request_count: u8,
    /// Currently staked tokens (earning rewards, if applicable)
    pub active_stake_amount: u64,
    /// Array of unstake requests with linear vesting
    pub unstake_requests: [UnstakeRequest; MAX_UNSTAKE_REQUESTS],
    /// Last update timestamp
    pub last_update_timestamp: i64,
    /// Padding for future use
    pub padding: [u64; 4],
}

impl UserStake {
    pub const LEN: usize = 8 + std::mem::size_of::<UserStake>();

    /// Get the total amount currently in linear unlock across all requests
    pub fn get_total_pending_unlock(&self) -> u64 {
        self.unstake_requests
            .iter()
            .map(|r| r.total_amount.saturating_sub(r.claimed_amount))
            .sum()
    }

    /// Get the total claimable amount across all requests
    pub fn get_total_claimable(&self, current_timestamp: i64) -> u64 {
        self.unstake_requests
            .iter()
            .map(|r| r.get_claimable_amount(current_timestamp))
            .sum()
    }

    /// Find an empty slot for a new unstake request
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.unstake_requests.iter().position(|r| r.is_empty())
    }

    /// Clean up fully claimed requests by swapping with the last active one
    pub fn cleanup_claimed_requests(&mut self) {
        let mut i = 0;
        while i < self.unstake_request_count as usize {
            if self.unstake_requests[i].is_fully_claimed() {
                // Swap with the last active request
                let last_idx = (self.unstake_request_count - 1) as usize;
                if i != last_idx {
                    self.unstake_requests.swap(i, last_idx);
                }
                // Clear the last slot
                self.unstake_requests[last_idx] = UnstakeRequest::default();
                self.unstake_request_count -= 1;
                // Don't increment i, check the swapped element
            } else {
                i += 1;
            }
        }
    }
}
