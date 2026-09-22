use xrpl::core::addresscodec::{
    is_valid_classic_address, is_valid_xaddress, xaddress_to_classic_address,
};

use crate::error::Error;

/// Validate a classic address or X-address.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The address to validate
    #[arg(long)]
    pub address: String,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let address = &self.address;

        if is_valid_classic_address(address) {
            println!("Valid classic address: {address}");
            Ok(())
        } else if is_valid_xaddress(address) {
            let (classic_address, tag, is_test) = xaddress_to_classic_address(address)?;
            println!("Valid X-address: {address}");
            println!("  Classic address: {classic_address}");
            println!("  Destination tag: {tag:?}");
            println!("  Test network: {is_test}");
            Ok(())
        } else {
            Err(Error::other(format!("Invalid address: {address}")))
        }
    }
}
