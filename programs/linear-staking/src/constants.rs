use anchor_lang::prelude::*;

#[constant]
pub const STAKE_VAULT_SEED: &[u8] = b"stake_vault";

#[constant]
pub const STAKE_VAULT_TOKEN_ACCOUNT_SEED: &[u8] = b"stake_vault_token_account";

#[constant]
pub const USER_STAKE_SEED: &[u8] = b"user_stake";

#[constant]
pub const TRANSFER_AUTHORITY_SEED: &[u8] = b"transfer_authority";

/// Default vesting period: 30 days in seconds
pub const DEFAULT_VESTING_PERIOD: i64 = 30 * 24 * 60 * 60; // 2,592,000 seconds

/// Token decimals (adjust based on your token)
pub const TOKEN_DECIMALS: u8 = 9;

/// Precision for calculations
pub const PRECISION: u128 = 1_000_000_000_000; // 10^12
