// These call `output::signed_tx_blob` and `output::submit_hint`, both now
// deprecated: prose on stdout is unpipeable, and the "To submit, use: ..." hint
// is what made copy-pasting a hex blob between two commands the documented
// workflow. The functions and these commands are deleted together by the
// removal PR, so the warning is silenced rather than chased here.
#![allow(deprecated)]

use std::borrow::Cow;

use xrpl::asynch::transaction::sign;
use xrpl::models::transactions::trust_set::TrustSet;
use xrpl::models::IssuedCurrencyAmount;
use xrpl::wallet::Wallet;

use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::output;

/// Sign a `TrustSet` for an issued currency.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The seed to use for signing
    #[arg(short, long)]
    pub seed: String,

    /// The issuer account
    #[arg(short, long)]
    pub issuer: String,

    /// The currency code (3-letter or 40-char hex)
    #[arg(short, long)]
    pub currency: String,

    /// The trust line limit (amount)
    #[arg(short, long)]
    pub limit: String,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let wallet = Wallet::new(&self.seed, 0)?;
        let limit_amount = IssuedCurrencyAmount::new(
            self.currency.clone().into(),
            self.issuer.clone().into(),
            self.limit.clone().into(),
        );

        let mut tx = TrustSet::builder(Cow::Owned(wallet.classic_address.clone()))
            .limit_amount(limit_amount)
            .build();

        sign(&mut tx, &wallet, false)?;
        let tx_blob = output::signed_tx_blob(&tx)?;
        output::submit_hint(&tx_blob, &self.network.url_or_mainnet());

        Ok(())
    }
}
