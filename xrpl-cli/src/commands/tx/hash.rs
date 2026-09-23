//! `xrpl tx hash` — the transaction ID. Offline, and signed transactions only.

use serde_json::Value;
use xrpl::core::binarycodec::encode;

use crate::commands::tx::args::TxInput;
use crate::commands::tx::io;
use crate::error::Error;

/// The prefix rippled hashes a signed transaction under (`TXN`).
const TRANSACTION_ID_PREFIX: [u8; 4] = (0x54584E00u32).to_be_bytes();

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub input: TxInput,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        for transaction in io::read_txs(&self.input.tx)? {
            crate::output::artifact(&transaction_id(&transaction)?)?;
        }

        Ok(())
    }
}

/// The 64-hex transaction ID of a signed transaction.
fn transaction_id(transaction: &Value) -> Result<String, Error> {
    let object = transaction
        .as_object()
        .ok_or_else(|| Error::other("expected a transaction object"))?;

    let signed = object.contains_key("TxnSignature")
        || object
            .get("Signers")
            .and_then(Value::as_array)
            .is_some_and(|signers| !signers.is_empty());

    if !signed {
        // The ID covers the signatures, so it does not exist until the last one
        // is attached. A ceremony quotes `tx digest` instead.
        return Err(Error::other(
            "unsigned: a transaction ID is not knowable until the last signature is attached. \
             Use `tx digest` to quote a pre-image to co-signers.",
        ));
    }

    let serialized = hex::decode(encode(transaction)?)?;
    let mut payload = Vec::with_capacity(serialized.len() + 4);
    payload.extend_from_slice(&TRANSACTION_ID_PREFIX);
    payload.extend_from_slice(&serialized);

    let digest = xrpl::core::keypairs::utils::sha512_first_half(&payload);

    Ok(hex::encode_upper(digest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_an_unsigned_transaction_has_no_id() {
        let tx = json!({"TransactionType": "Payment", "SigningPubKey": ""});
        let message = transaction_id(&tx).unwrap_err().to_string();

        assert!(message.contains("not knowable"), "{message}");
    }

    #[test]
    fn test_an_empty_signers_array_is_still_unsigned() {
        let tx = json!({"TransactionType": "Payment", "SigningPubKey": "", "Signers": []});
        assert!(transaction_id(&tx).is_err());
    }
}
