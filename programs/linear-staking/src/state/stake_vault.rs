use anchor_lang::prelude::*;

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct StakePermissions {
    pub allow_deposits: bool,
    pub allow_withdrawals: bool,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct StakeStats {
    /// Tokens currently actively staked
    pub active_amount: u64,
    /// Tokens that are in the linear unlock period (pending withdrawal)
    pub pending_unlock: u64,
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
    /// The mint of the token being staked
    pub token_mint: Pubkey,
    /// The vault's token account
    pub vault_token_account: Pubkey,
    /// Admin authority that can update vault settings
    pub admin: Pubkey,
    /// Permissions for deposits/withdrawals
    pub permissions: StakePermissions,
    /// Linear vesting period in seconds (default 30 days = 2,592,000 seconds)
    pub vesting_period: i64,
    /// Global stake statistics
    pub stake_stats: StakeStats,
    /// Padding for future use
    pub padding: [u64; 8],
}

impl StakeVault {
    pub const LEN: usize = 8 + std::mem::size_of::<StakeVault>();
}
