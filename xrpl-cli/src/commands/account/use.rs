//! `xrpl account use` — choose the default account.

use crate::error::Error;
use crate::output;
use crate::store::Store;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The account alias. Omit with --clear to unset the default.
    pub alias: Option<String>,

    /// Unset the default.
    #[arg(long, conflicts_with = "alias")]
    pub clear: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;

        if self.clear {
            store.set_default_account(None)?;
            output::note("no default account");
            return Ok(());
        }

        let alias = self
            .alias
            .as_deref()
            .ok_or_else(|| Error::other("name an account, or pass --clear"))?;

        let record = store.account(alias)?;
        store.set_default_account(Some(alias))?;

        output::note(format!(
            "default account is now {alias} ({})",
            record.address
        ));

        // The default is only ever *inherited* off-mainnet, so saying so at the
        // point of setting it beats discovering it at the point of signing.
        if record.is_mainnet() {
            output::warn(
                "this account is on mainnet, so commands will still require an explicit \
                 --account: an inherited account decides who pays.",
            );
        }

        Ok(())
    }
}
