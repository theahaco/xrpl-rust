use xrpl::ledger::{get_fee, FeeType};

use crate::client;
use crate::commands::global::NetworkArgs;
use crate::error::Error;

/// Report the node's current open-ledger fee.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let runtime = client::runtime()?;
        let client = client::json_rpc(&self.network.url_or_mainnet())?;

        // get_fee is sync but the client's transport needs a reactor.
        match runtime.block_on(async { get_fee(&client, None, Some(FeeType::Open)) }) {
            Ok(fee) => {
                println!("Current network fee: {fee} drops");
                Ok(())
            }
            Err(error) => Err(Error::Helper(error)),
        }
    }
}
