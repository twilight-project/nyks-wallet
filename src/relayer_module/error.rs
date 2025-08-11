use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrderWalletError {
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("account {0} not on-chain or wrong IO type")]
    BadAccountState(u64),
    #[error("order status {0} not acceptable for this operation")]
    InvalidOrderStatus(String),
    #[error("missing request id for account {0}")]
    MissingRequestId(u64),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, OrderWalletError>;
