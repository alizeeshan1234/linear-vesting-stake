use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Deposits are currently disabled")]
    DepositsDisabled,

    #[msg("Withdrawals are currently disabled")]
    WithdrawalsDisabled,

    #[msg("Invalid amount provided")]
    InvalidAmount,

    #[msg("Insufficient staked balance")]
    InsufficientBalance,

    #[msg("Maximum number of unstake requests reached")]
    MaxUnstakeRequestsReached,

    #[msg("No vested tokens available to claim")]
    NoVestedTokens,

    #[msg("Invalid unstake request ID")]
    InvalidUnstakeRequestId,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Unauthorized access")]
    Unauthorized,

    #[msg("No rewards available to claim")]
    NoRewardsToClaim,

    #[msg("No active stake to distribute rewards")]
    NoActiveStake,

    #[msg("No pending rewards to distribute")]
    NoPendingRewards,

    #[msg("Deposits not allowed")]
    DepositsNotAllowed,

    #[msg("No claimable amount available")]
    NoClaimableAmount,

    #[msg("Invalid request index")]
    InvalidRequestIndex,

    #[msg("No amount to cancel")]
    NoAmountToCancel,

    NumericalOverflow,

    #[msg("Vault is paused")]
    VaultPaused,

    #[msg("Vault is already paused")]
    VaultAlreadyPaused,

    #[msg("Vault is not paused")]
    NotPaused,

    #[msg("Vault must be paused for emergency withdraw")]
    VaultNotPaused,

    #[msg("Invalid vesting period")]
    InvalidVestingPeriod,

    #[msg("Insufficient vault balance")]
    InsufficientVaultBalance,
}
