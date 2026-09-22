use thiserror_no_std::Error;

/// Every command returns this. Variants mirror the library error types the CLI
/// can encounter, plus [`Error::Other`] for CLI-level problems (an unknown flag
/// name, an unsupported transaction type) that have no library counterpart.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Wallet error: {0}")]
    Wallet(#[from] xrpl::wallet::exceptions::XRPLWalletException),
    #[error("Client error: {0}")]
    Client(#[from] xrpl::asynch::clients::exceptions::XRPLClientException),
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Helper error: {0}")]
    Helper(#[from] xrpl::asynch::exceptions::XRPLHelperException),
    #[error("Core error: {0}")]
    Core(#[from] xrpl::core::exceptions::XRPLCoreException),
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Build an [`Error::Other`] from anything printable.
    pub fn other(message: impl std::fmt::Display) -> Self {
        Error::Other(message.to_string())
    }
}
