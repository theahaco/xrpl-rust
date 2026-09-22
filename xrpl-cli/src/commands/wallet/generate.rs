use bip39::{Language, Mnemonic};
use xrpl::wallet::Wallet;

use crate::error::Error;

/// Generate a new wallet, optionally from a BIP39 mnemonic.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Save the wallet to a file
    #[arg(long)]
    pub save: bool,

    /// Generate a BIP39 mnemonic phrase
    #[arg(long)]
    pub mnemonic: bool,

    /// Number of words for the mnemonic (12, 15, 18, 21, 24)
    #[arg(long, default_value_t = 12)]
    pub words: usize,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        if self.mnemonic {
            let mut rng = rand::thread_rng();
            let mnemonic = Mnemonic::generate_in_with(&mut rng, Language::English, self.words)
                .map_err(|_| Error::other("Invalid word count (must be 12, 15, 18, 21, or 24)"))?;
            let seed = mnemonic.to_seed("");
            let phrase = mnemonic.words().collect::<Vec<_>>().join(" ");

            println!(
                "Generated wallet with mnemonic:\nMnemonic: {}\nSeed: {}",
                phrase,
                hex::encode(seed)
            );
        } else {
            let wallet = Wallet::create(None)?;
            println!("Generated wallet: {wallet:#?}");

            if self.save {
                println!("Saving wallet functionality not implemented yet");
            }
        }

        Ok(())
    }
}
