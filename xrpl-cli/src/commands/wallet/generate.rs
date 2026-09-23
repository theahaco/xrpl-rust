//! `xrpl wallet generate` — make a new key pair.

use serde_json::json;
use xrpl::wallet::Wallet;

use xrpl::signer::RawSigner;

use crate::error::Error;
use crate::output;

/// Generate a new wallet.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Include the seed in the output.
    ///
    /// Without this the seed is generated and thrown away, which is only useful
    /// for seeing what an address looks like. Redirect this to a file yourself
    /// if you mean to keep it: `xrpl wallet generate --show-secret > k` with a
    /// `umask` you chose.
    #[arg(long)]
    pub show_secret: bool,

    /// Removed. The CLI does not write secrets to disk.
    #[arg(long, hide = true)]
    pub save: bool,

    /// Removed along with the BIP39 dependency.
    #[arg(long, hide = true)]
    pub mnemonic: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        if self.save {
            // Writing a cleartext seed file is the one artifact this CLI will
            // not produce. Mode 0600 survives neither a backup, a dotfiles
            // sync, a container layer nor a stolen disk, and a user who asked
            // for `--save` would reasonably believe otherwise.
            return Err(Error::other(
                "--save is not supported: the CLI does not write secrets to disk. \
                 Redirect --show-secret yourself, with a umask you chose.",
            ));
        }

        if self.mnemonic {
            // It printed a BIP39 phrase and a 64-byte hex seed that `Wallet::new`
            // cannot consume — it wants a base58 `s…` family seed — and this
            // crate has no BIP32/BIP44 convention to bridge them. The 16-byte
            // family seed *is* the backup.
            return Err(Error::other(
                "--mnemonic is not supported: this crate has no derivation-path convention, \
                 and the phrase it used to print produced a seed nothing here could read.",
            ));
        }

        let wallet = Wallet::create(None)?;

        // JSON on stdout, so a script can `jq` it. The previous output was
        // `Wallet`'s redacted `Debug`, which showed the user nothing usable at
        // all — a generated wallet whose secret was unrecoverable the moment
        // the process exited.
        let mut value = json!({
            "classic_address": wallet.classic_address,
            "public_key": wallet.public_key,
            "algorithm": format!("{:?}", wallet.algorithm()),
        });

        if self.show_secret {
            value["seed"] = json!(wallet.seed.as_str());
        } else {
            output::note(
                "the seed was not printed and is now gone. Pass --show-secret to keep it.",
            );
        }

        output::artifact(&value)
    }
}
