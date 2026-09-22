use serde_json::Value;
use xrpl::asynch::transaction::sign;
use xrpl::models::transactions::{
    account_set::AccountSet, offer_cancel::OfferCancel, offer_create::OfferCreate,
    payment::Payment, trust_set::TrustSet,
};
use xrpl::wallet::Wallet;

use crate::error::Error;
use crate::output;

/// Sign a transaction given as JSON and print its binary blob.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The seed to use for signing
    #[arg(short, long)]
    pub seed: String,

    /// The transaction type (Payment, AccountSet, etc.)
    #[arg(short, long)]
    pub r#type: String,

    /// The transaction JSON
    #[arg(short, long)]
    pub json: String,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let wallet = Wallet::new(&self.seed, 0)?;
        let json: Value = serde_json::from_str(&self.json)?;

        // Each arm parses into its own model so the signature covers the same
        // fields rippled will validate.
        match self.r#type.to_lowercase().as_str() {
            "payment" => {
                let mut tx: Payment = serde_json::from_value(json)?;
                sign(&mut tx, &wallet, false)?;
                output::signed_tx_blob(&tx)?;
            }
            "accountset" => {
                let mut tx: AccountSet = serde_json::from_value(json)?;
                sign(&mut tx, &wallet, false)?;
                output::signed_tx_blob(&tx)?;
            }
            "offercreate" => {
                let mut tx: OfferCreate = serde_json::from_value(json)?;
                sign(&mut tx, &wallet, false)?;
                output::signed_tx_blob(&tx)?;
            }
            "offercancel" => {
                let mut tx: OfferCancel = serde_json::from_value(json)?;
                sign(&mut tx, &wallet, false)?;
                output::signed_tx_blob(&tx)?;
            }
            "trustset" => {
                let mut tx: TrustSet = serde_json::from_value(json)?;
                sign(&mut tx, &wallet, false)?;
                output::signed_tx_blob(&tx)?;
            }
            other => {
                return Err(Error::other(format!(
                    "Unsupported transaction type: {other}"
                )))
            }
        }

        Ok(())
    }
}
