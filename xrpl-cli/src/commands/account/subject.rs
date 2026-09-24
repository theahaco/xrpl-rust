//! Naming the account a ledger query is about.
//!
//! One flag, `--account`, taking an alias or a literal r-address. `--address`
//! remains as a hidden alias for one minor so existing invocations keep working,
//! but it is not what the help shows.
//!
//! The resolution here is a single file read: no credential store, no device,
//! no network, no prompt. That is a property with a test behind it, because it
//! is what stops a listing from prompting once per row — the failure mode that
//! made a credential-store-backed CLI unusable elsewhere.

use crate::error::Error;
use crate::store::{resolve, Store};

/// `--account <alias|r-address>` on a ledger query.
#[derive(Debug, Clone, clap::Args)]
pub struct AccountArg {
    /// The account: a recorded alias, or a literal r-address.
    ///
    /// An alias resolves through the local record. A literal address is used
    /// as given. Anything that parses as a seed is refused — this names an
    /// account, never a key.
    #[arg(long, value_name = "ALIAS_OR_ADDRESS", alias = "address")]
    pub account: String,
}

impl AccountArg {
    /// The address this names.
    pub fn address(&self) -> Result<String, Error> {
        let store = Store::from_env()?;
        Ok(resolve::account(&store, Some(&self.account))?.address)
    }
}
