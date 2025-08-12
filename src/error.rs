use anyhow::Error as AnyhowError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("account {0} not on-chain or wrong IO type")]
    BadAccountState(u64),
    #[error("order status {0} not acceptable for this operation")]
    InvalidOrderStatus(String),
    #[error("missing request id for account {0}")]
    MissingRequestId(u64),
    #[error("failed to create relayer client: {0}")]
    RelayerClient(String),
    #[error("wallet creation failed: {0}")]
    WalletCreation(String),
    #[error("failed to update wallet balance: {0}")]
    WalletBalanceUpdate(String),
    #[error("failed to fetch wallet account info: {0}")]
    WalletAccountInfo(String),
    #[error("failed to build transaction: {0}")]
    TxBuild(String),
    #[error("transaction broadcast failed (code {code}) for tx {tx_hash}")]
    TxBroadcastFailed { tx_hash: String, code: u32 },
    #[error(
        "failed to fetch UTXO details after {attempts} attempts for IO type {io_type}: {source}"
    )]
    FetchUtxoFailed {
        attempts: u32,
        io_type: String,
        #[source]
        source: AnyhowError,
    },
    #[error("RPC request failed: {0}")]
    RpcRequest(String),
    #[error("failed to create trader order: {0}")]
    CreateTraderOrder(String),
    #[error("failed to close trader order: {0}")]
    CloseTraderOrder(String),
    #[error("failed to cancel trader order: {0}")]
    CancelTraderOrder(String),
    #[error("failed to create lend order: {0}")]
    CreateLendOrder(String),
    #[error("failed to close lend order: {0}")]
    CloseLendOrder(String),
    #[error("zk account DB error: {0}")]
    ZkAccountDb(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("password prompt failed: {0}")]
    PasswordPrompt(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    #[error("zk account seed not found")]
    ZkAccountSeedNotFound(String),
}

pub type Result<T> = std::result::Result<T, WalletError>;
