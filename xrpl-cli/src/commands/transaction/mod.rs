pub mod nft_burn;
pub mod nft_mint;
pub mod sign;
pub mod submit;
pub mod trust_set;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Sign a transaction
    Sign(sign::Cmd),

    /// Submit a transaction
    Submit(submit::Cmd),

    /// Set or modify a trust line
    TrustSet(trust_set::Cmd),

    /// Mint an NFT (XLS-20)
    NftMint(nft_mint::Cmd),

    /// Burn an NFT (XLS-20)
    NftBurn(nft_burn::Cmd),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::Sign(cmd) => cmd.run(),
            Cmd::Submit(cmd) => cmd.run(),
            Cmd::TrustSet(cmd) => cmd.run(),
            Cmd::NftMint(cmd) => cmd.run(),
            Cmd::NftBurn(cmd) => cmd.run(),
        }
    }
}
