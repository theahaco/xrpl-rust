//! Shared body for `account set-flag` and `account clear-flag`: the two differ
//! only in which `AccountSet` field the parsed flag lands in.

use std::borrow::Cow;
use std::str::FromStr;

use xrpl::asynch::transaction::sign;
use xrpl::models::transactions::account_set::{AccountSet, AccountSetFlag};
use xrpl::wallet::Wallet;

use crate::error::Error;
use crate::output;

/// Which side of `AccountSet` the flag is written to.
#[derive(Debug, Clone, Copy)]
pub enum Action {
    Set,
    Clear,
}

/// Build the `AccountSet` this command signs.
fn transaction<'a>(address: &str, flag: &str, action: Action) -> Result<AccountSet<'a>, Error> {
    let flag = AccountSetFlag::from_str(flag)
        .map_err(|_| Error::other(format!("Invalid flag: {flag}")))?;

    let builder = AccountSet::builder(Cow::Owned(address.to_string()));

    Ok(match action {
        Action::Set => builder.set_flag(flag).build(),
        Action::Clear => builder.clear_flag(flag).build(),
    })
}

pub fn run(seed: &str, flag: &str, url: &str, action: Action) -> Result<(), Error> {
    let wallet = Wallet::new(seed, 0)?;
    let mut tx = transaction(&wallet.classic_address, flag, action)?;

    sign(&mut tx, &wallet, false)?;
    let tx_blob = output::signed_tx_blob(&tx)?;
    output::submit_hint(&tx_blob, url);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";

    #[test]
    fn set_writes_set_flag() {
        let tx = transaction(ADDRESS, "asfRequireAuth", Action::Set).unwrap();
        assert_eq!(tx.set_flag, Some(AccountSetFlag::AsfRequireAuth));
        assert_eq!(tx.clear_flag, None);
    }

    #[test]
    fn clear_writes_clear_flag() {
        let tx = transaction(ADDRESS, "asfRequireAuth", Action::Clear).unwrap();
        assert_eq!(tx.clear_flag, Some(AccountSetFlag::AsfRequireAuth));
        assert_eq!(tx.set_flag, None);
    }

    #[test]
    fn unknown_flag_is_rejected() {
        assert!(transaction(ADDRESS, "asfNotAFlag", Action::Set).is_err());
    }
}
