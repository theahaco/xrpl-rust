use std::process::ExitCode;

use thiserror_no_std::Error;

/// Every command returns this.
///
/// `#[non_exhaustive]`, because the account/key/signer work adds variants and a
/// downstream exhaustive `match` should not break each time one arrives.
///
/// New failures get a typed variant. [`Error::Other`] is for CLI-level problems
/// that genuinely have no structure to them; it is not a place to put a failure
/// a caller might want to branch on.
#[derive(Debug, Error)]
#[non_exhaustive]
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
    #[error("Hex error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("Helper error: {0}")]
    Helper(#[from] xrpl::asynch::exceptions::XRPLHelperException),
    #[error("Core error: {0}")]
    Core(#[from] xrpl::core::exceptions::XRPLCoreException),
    #[error("Signer error: {0}")]
    Signer(#[from] SignerError),
    /// A batch was folded with the wrong number of inner transactions.
    #[error("a Batch holds 2 to 8 inner transactions, got {found}")]
    BatchSize { found: usize },

    /// More batch co-signers than the ledger accepts.
    #[error("a Batch carries at most 24 BatchSigners (three per inner transaction), got {found}")]
    BatchSignerCount { found: usize },

    /// An inner transaction is not authorized by exactly one thing.
    #[error("an inner transaction must carry exactly one of a non-zero Sequence or a TicketSequence: {detail}")]
    BatchInnerSequence { detail: String },

    /// A fold crossed networks.
    ///
    /// Replay protection: the outer signature vouches for every inner ID it
    /// commits to, so a batch is the easiest place to fold in a transaction
    /// built for another chain.
    #[error("the batch crosses networks: {detail}")]
    BatchNetworkMismatch { detail: String },

    /// The node answered, and its answer was an error.
    ///
    /// Distinct from [`Error::Client`], which is a transport failure, and from
    /// [`Error::Helper`], which is a transaction the ledger rejected. This is a
    /// well-formed reply saying the request could not be served — `lgrNotFound`,
    /// `actNotFound` — and it shares exit 2 with the transport failures because
    /// a caller's remedy is the same: ask differently, or ask elsewhere.
    #[error("the node answered with {code}: {message}")]
    Node { code: String, message: String },

    #[error("funding for {address} was not confirmed within {seconds}s; it may still arrive. Check `xrpl account info --account {address}` with the same --network/--url before requesting again")]
    FundingTimeout { address: String, seconds: u64 },

    #[error("the node returned an invalid or unvalidated account balance for {address}; funding is not confirmed")]
    FundingResponse { address: String },

    #[error("{0}")]
    Other(String),
}

/// Why a signing backend could not produce a signature.
///
/// Separate from [`Error`] because these are the failures a script must be able
/// to tell apart: a signature the user deliberately cancelled must not look like
/// an unreachable node, or every retry wrapper written against this CLI will
/// retry the cancellation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SignerError {
    /// No account or key record by that name.
    #[error("No such {kind}: {name}")]
    NotFound { kind: &'static str, name: String },
    /// The record exists; the secret it points at is not reachable from here —
    /// a credential-store entry made on another machine, a device that is not
    /// plugged in. Different remedy from "no such key", so a different code.
    /// Carries its own sentence and adds nothing to it. Every construction
    /// site passes a full clause naming the signer and the reason, so the
    /// `Signer {0} is unavailable on this machine` this used to wrap them in
    /// spliced the two together: "Signer alice is watch-only: it has no keys
    /// is unavailable on this machine". `Error::Signer` already prefixes
    /// "Signer error", and the exit code is what tells this apart from
    /// [`SignerError::Backend`], not the prose.
    #[error("{0}")]
    Unavailable(String),
    /// A human declined, or the prompt could not be shown.
    #[error("Declined: {0}")]
    Declined(String),
    /// The backend exists in the CLI's vocabulary but was not compiled in.
    ///
    /// A runtime error rather than a missing enum variant: Cargo features are
    /// additive, so the shape of a public type must not depend on the feature
    /// set (#14).
    #[error("Backend {0} is not compiled into this binary")]
    BackendNotEnabled(&'static str),
    #[error("{0}")]
    Backend(String),
}

impl Error {
    /// Build an [`Error::Other`] from anything printable.
    pub fn other(message: impl std::fmt::Display) -> Self {
        Error::Other(message.to_string())
    }

    /// The process exit status for this error.
    ///
    /// The canonical table, owned by #16. Codes `0`-`3` are the `tx` pipeline's
    /// convention and are fixed; `4`-`6` are the account/key/signer work's and
    /// are allocated above them so nothing already asserted moves.
    ///
    /// | code | meaning |
    /// |------|---------|
    /// | 0 | success |
    /// | 1 | usage / parse |
    /// | 2 | network / client |
    /// | 3 | validated ledger-level failure |
    /// | 4 | configuration not found |
    /// | 5 | user declined |
    /// | 6 | signer unavailable |
    ///
    /// Everything mapped to `ExitCode::FAILURE` before this, so a bad flag, an
    /// unreachable node and a `tec` result were indistinguishable to a caller.
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(match self {
            Error::UrlParse(_)
            | Error::Json(_)
            | Error::Other(_)
            | Error::BatchSize { .. }
            | Error::BatchSignerCount { .. }
            | Error::BatchInnerSequence { .. }
            | Error::BatchNetworkMismatch { .. } => exit::USAGE,
            Error::Client(_)
            | Error::Node { .. }
            | Error::FundingTimeout { .. }
            | Error::FundingResponse { .. } => exit::NETWORK,
            Error::Wallet(_) | Error::Core(_) | Error::Io(_) | Error::Hex(_) | Error::Toml(_) => {
                exit::USAGE
            }
            Error::Helper(_) => exit::LEDGER,
            Error::Signer(signer) => match signer {
                SignerError::NotFound { .. } => exit::CONFIG_NOT_FOUND,
                SignerError::Declined(_) => exit::DECLINED,
                SignerError::Unavailable(_) | SignerError::BackendNotEnabled(_) => {
                    exit::SIGNER_UNAVAILABLE
                }
                SignerError::Backend(_) => exit::USAGE,
            },
        })
    }
}

/// The exit-code table, as named constants so tests can assert against the
/// names rather than repeating the numbers.
pub mod exit {
    /// Success.
    pub const SUCCESS: u8 = 0;
    /// A bad flag, malformed JSON, a rejected field name.
    pub const USAGE: u8 = 1;
    /// The node was unreachable, or the transport failed.
    pub const NETWORK: u8 = 2;
    /// A transaction reached a validated ledger and failed there.
    pub const LEDGER: u8 = 3;
    /// No such account alias or key id.
    pub const CONFIG_NOT_FOUND: u8 = 4;
    /// A human cancelled a confirmation or a passphrase prompt.
    pub const DECLINED: u8 = 5;
    /// The record is present; the secret is not reachable from here.
    pub const SIGNER_UNAVAILABLE: u8 = 6;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(error: Error) -> ExitCode {
        error.exit_code()
    }

    #[test]
    fn test_each_signer_failure_gets_its_own_code() {
        // A retry wrapper branches on these. Collapsing them is how a
        // deliberately cancelled signature gets retried.
        assert_eq!(
            format!(
                "{:?}",
                code(
                    SignerError::NotFound {
                        kind: "account",
                        name: "alice".into()
                    }
                    .into()
                )
            ),
            format!("{:?}", ExitCode::from(exit::CONFIG_NOT_FOUND))
        );
        assert_eq!(
            format!(
                "{:?}",
                code(SignerError::Declined("cancelled".into()).into())
            ),
            format!("{:?}", ExitCode::from(exit::DECLINED))
        );
        assert_eq!(
            format!(
                "{:?}",
                code(SignerError::Unavailable("alice".into()).into())
            ),
            format!("{:?}", ExitCode::from(exit::SIGNER_UNAVAILABLE))
        );
        assert_eq!(
            format!(
                "{:?}",
                code(SignerError::BackendNotEnabled("secure-store").into())
            ),
            format!("{:?}", ExitCode::from(exit::SIGNER_UNAVAILABLE))
        );
    }

    #[test]
    fn test_the_table_has_no_duplicates_and_no_gaps() {
        let table = [
            exit::SUCCESS,
            exit::USAGE,
            exit::NETWORK,
            exit::LEDGER,
            exit::CONFIG_NOT_FOUND,
            exit::DECLINED,
            exit::SIGNER_UNAVAILABLE,
        ];

        // Codes 0-3 are fixed by the `tx` pipeline convention and must never be
        // renumbered; 4-6 are allocated above them.
        assert_eq!(table, [0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_a_usage_error_is_not_a_network_error() {
        assert_ne!(
            format!("{:?}", code(Error::other("bad flag"))),
            format!("{:?}", ExitCode::from(exit::NETWORK))
        );
    }
}
