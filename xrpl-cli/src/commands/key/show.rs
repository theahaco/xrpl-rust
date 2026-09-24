//! `xrpl key show` — one key record.

use crate::error::Error;
use crate::store::Store;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The key id.
    pub id: String,

    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        let record = store.key(&self.id)?;
        let described = super::describe(&self.id, &record);

        if self.json {
            return crate::output::artifact(&described);
        }

        println!("id              {}", self.id);
        println!("public key      {}", record.public_key);
        println!("algorithm       {}", record.algorithm);
        println!("address         {}", record.classic_address);
        println!("secret          {}", record.source.label());

        Ok(())
    }
}
