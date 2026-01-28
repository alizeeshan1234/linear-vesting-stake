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
}
