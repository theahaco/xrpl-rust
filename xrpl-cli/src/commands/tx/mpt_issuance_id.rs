//! `xrpl tx mpt-issuance-id` — derive an MPT issuance ID offline.
//!
//! The ID is `Sequence` (4 bytes, big-endian) followed by the issuer's 20-byte
//! AccountID. A `submit` response carries no metadata, so this is how a script
//! learns the ID of an issuance it just created without a second lookup.

use xrpl::core::addresscodec::decode_classic_address;

use crate::error::Error;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The issuing account.
    #[arg(long, value_name = "R_ADDRESS")]
    pub account: String,

    /// The `Sequence` of the `MPTokenIssuanceCreate` that created it.
    #[arg(long)]
    pub sequence: u32,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        crate::output::artifact(&issuance_id(&self.account, self.sequence)?)
    }
}

fn issuance_id(account: &str, sequence: u32) -> Result<String, Error> {
    let account_id = decode_classic_address(account)?;

    let mut id = Vec::with_capacity(24);
    id.extend_from_slice(&sequence.to_be_bytes());
    id.extend_from_slice(&account_id);

    Ok(hex::encode_upper(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENESIS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";

    #[test]
    fn test_the_id_is_48_hex_characters() {
        let id = issuance_id(GENESIS, 1).expect("derives");
        assert_eq!(id.len(), 48, "4 bytes of sequence plus a 20-byte AccountID");
    }

    #[test]
    fn test_the_sequence_is_big_endian_and_leads() {
        let id = issuance_id(GENESIS, 1).expect("derives");
        assert!(id.starts_with("00000001"), "{id}");
    }

    #[test]
    fn test_the_issuer_account_id_follows_the_sequence() {
        let id = issuance_id(GENESIS, 7).expect("derives");
        let account_id = hex::encode_upper(decode_classic_address(GENESIS).expect("decodes"));

        assert_eq!(&id[8..], account_id);
    }

    #[test]
    fn test_a_malformed_address_is_an_error_not_a_panic() {
        assert!(issuance_id("not-an-address", 1).is_err());
    }
}
