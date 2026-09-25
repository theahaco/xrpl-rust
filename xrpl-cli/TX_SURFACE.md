# `xrpl tx new` surface

Generated from the vendored `definitions.json`. Regenerate with
`cargo run -p xrpl-cli --bin tx-surface > xrpl-cli/TX_SURFACE.md`;
CI fails if this file and the definitions disagree.

An `AccountID` flag takes a literal address or the alias of a local
account, and so does the address half of `--signer-entry`, the issuer in
`--amount 100/USD/<issuer>`, and `--field <Name>=` for an AccountID field.
A valid address is always taken literally; any other value must name an
account record, and an unknown name is refused naming its flag.

## AMMBid — `xrpl tx new amm-bid`

| flag | type | required |
|---|---|---|
| `--asset` | Issue | yes |
| `--asset2` | Issue | yes |
| `--bid-min` | Amount |  |
| `--bid-max` | Amount |  |
| `--auth-accounts` | STArray |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## AMMClawback — `xrpl tx new amm-clawback`

| flag | type | required |
|---|---|---|
| `--holder` | AccountID | yes |
| `--asset` | Issue | yes |
| `--asset2` | Issue | yes |
| `--amount` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfClawTwoAssets` (1), `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## AMMCreate — `xrpl tx new amm-create`

| flag | type | required |
|---|---|---|
| `--amount` | Amount | yes |
| `--amount2` | Amount | yes |
| `--trading-fee` | UInt16 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## AMMDelete — `xrpl tx new amm-delete`

| flag | type | required |
|---|---|---|
| `--asset` | Issue | yes |
| `--asset2` | Issue | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## AMMDeposit — `xrpl tx new amm-deposit`

| flag | type | required |
|---|---|---|
| `--asset` | Issue | yes |
| `--asset2` | Issue | yes |
| `--amount` | Amount |  |
| `--amount2` | Amount |  |
| `--e-price` | Amount |  |
| `--lp-token-out` | Amount |  |
| `--trading-fee` | UInt16 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfLPToken` (65536), `tfLimitLPToken` (4194304), `tfOneAssetLPToken` (2097152), `tfSingleAsset` (524288), `tfTwoAsset` (1048576), `tfTwoAssetIfEmpty` (8388608)

## AMMVote — `xrpl tx new amm-vote`

| flag | type | required |
|---|---|---|
| `--asset` | Issue | yes |
| `--asset2` | Issue | yes |
| `--trading-fee` | UInt16 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## AMMWithdraw — `xrpl tx new amm-withdraw`

| flag | type | required |
|---|---|---|
| `--asset` | Issue | yes |
| `--asset2` | Issue | yes |
| `--amount` | Amount |  |
| `--amount2` | Amount |  |
| `--e-price` | Amount |  |
| `--lp-token-in` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfLPToken` (65536), `tfLimitLPToken` (4194304), `tfOneAssetLPToken` (2097152), `tfOneAssetWithdrawAll` (262144), `tfSingleAsset` (524288), `tfTwoAsset` (1048576), `tfWithdrawAll` (131072)

## AccountDelete — `xrpl tx new account-delete`

| flag | type | required |
|---|---|---|
| `--destination` | AccountID | yes |
| `--destination-tag` | UInt32 |  |
| `--credential-ids` | Vector256 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## AccountSet — `xrpl tx new account-set`

| flag | type | required |
|---|---|---|
| `--email-hash` | Hash128 |  |
| `--wallet-locator` | Hash256 |  |
| `--wallet-size` | UInt32 |  |
| `--message-key` | Blob |  |
| `--domain` | Blob |  |
| `--transfer-rate` | UInt32 |  |
| `--tick-size` | UInt8 |  |
| `--nftoken-minter` | AccountID |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfAllowXRP` (2097152), `tfDisallowXRP` (1048576), `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfOptionalAuth` (524288), `tfOptionalDestTag` (131072), `tfRequireAuth` (262144), `tfRequireDestTag` (65536)

## Batch — `xrpl tx new batch`

| flag | type | required |
|---|---|---|
| `--raw-transactions` | STArray | yes |
| `--batch-signers` | STArray |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfAllOrNothing` (65536), `tfFullyCanonicalSig` (2147483648), `tfIndependent` (524288), `tfInnerBatchTxn` (1073741824), `tfOnlyOne` (131072), `tfUntilFailure` (262144)

## CheckCancel — `xrpl tx new check-cancel`

| flag | type | required |
|---|---|---|
| `--check-id` | Hash256 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## CheckCash — `xrpl tx new check-cash`

| flag | type | required |
|---|---|---|
| `--check-id` | Hash256 | yes |
| `--amount` | Amount |  |
| `--deliver-min` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## CheckCreate — `xrpl tx new check-create`

| flag | type | required |
|---|---|---|
| `--destination` | AccountID | yes |
| `--send-max` | Amount | yes |
| `--expiration` | UInt32 |  |
| `--destination-tag` | UInt32 |  |
| `--invoice-id` | Hash256 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## Clawback — `xrpl tx new clawback`

| flag | type | required |
|---|---|---|
| `--amount` | Amount | yes |
| `--holder` | AccountID |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## ConfidentialMPTClawback — `xrpl tx new confidential-mpt-clawback`

| flag | type | required |
|---|---|---|
| `--mptoken-issuance-id` | Hash192 | yes |
| `--holder` | AccountID | yes |
| `--mpt-amount` | UInt64 | yes |
| `--zk-proof` | Blob | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## ConfidentialMPTConvert — `xrpl tx new confidential-mpt-convert`

| flag | type | required |
|---|---|---|
| `--mptoken-issuance-id` | Hash192 | yes |
| `--mpt-amount` | UInt64 | yes |
| `--holder-encryption-key` | Blob |  |
| `--holder-encrypted-amount` | Blob | yes |
| `--issuer-encrypted-amount` | Blob | yes |
| `--auditor-encrypted-amount` | Blob |  |
| `--blinding-factor` | Hash256 | yes |
| `--zk-proof` | Blob |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## ConfidentialMPTConvertBack — `xrpl tx new confidential-mpt-convert-back`

| flag | type | required |
|---|---|---|
| `--mptoken-issuance-id` | Hash192 | yes |
| `--mpt-amount` | UInt64 | yes |
| `--holder-encrypted-amount` | Blob | yes |
| `--issuer-encrypted-amount` | Blob | yes |
| `--auditor-encrypted-amount` | Blob |  |
| `--blinding-factor` | Hash256 | yes |
| `--zk-proof` | Blob | yes |
| `--balance-commitment` | Blob | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## ConfidentialMPTMergeInbox — `xrpl tx new confidential-mpt-merge-inbox`

| flag | type | required |
|---|---|---|
| `--mptoken-issuance-id` | Hash192 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## ConfidentialMPTSend — `xrpl tx new confidential-mpt-send`

| flag | type | required |
|---|---|---|
| `--mptoken-issuance-id` | Hash192 | yes |
| `--destination` | AccountID | yes |
| `--destination-tag` | UInt32 |  |
| `--sender-encrypted-amount` | Blob | yes |
| `--destination-encrypted-amount` | Blob | yes |
| `--issuer-encrypted-amount` | Blob | yes |
| `--auditor-encrypted-amount` | Blob |  |
| `--zk-proof` | Blob | yes |
| `--amount-commitment` | Blob | yes |
| `--balance-commitment` | Blob | yes |
| `--credential-ids` | Vector256 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## CredentialAccept — `xrpl tx new credential-accept`

| flag | type | required |
|---|---|---|
| `--issuer` | AccountID | yes |
| `--credential-type` | Blob | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## CredentialCreate — `xrpl tx new credential-create`

| flag | type | required |
|---|---|---|
| `--subject` | AccountID | yes |
| `--credential-type` | Blob | yes |
| `--expiration` | UInt32 |  |
| `--uri` | Blob |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## CredentialDelete — `xrpl tx new credential-delete`

| flag | type | required |
|---|---|---|
| `--subject` | AccountID |  |
| `--issuer` | AccountID |  |
| `--credential-type` | Blob | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## DIDDelete — `xrpl tx new did-delete`

| flag | type | required |
|---|---|---|
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## DIDSet — `xrpl tx new did-set`

| flag | type | required |
|---|---|---|
| `--did-document` | Blob |  |
| `--uri` | Blob |  |
| `--data` | Blob |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## DelegateSet — `xrpl tx new delegate-set`

| flag | type | required |
|---|---|---|
| `--authorize` | AccountID | yes |
| `--permissions` | STArray | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## DepositPreauth — `xrpl tx new deposit-preauth`

| flag | type | required |
|---|---|---|
| `--authorize` | AccountID |  |
| `--unauthorize` | AccountID |  |
| `--authorize-credentials` | STArray |  |
| `--unauthorize-credentials` | STArray |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## EnableAmendment — `xrpl tx new enable-amendment`

| flag | type | required |
|---|---|---|
| `--ledger-sequence` | UInt32 | yes |
| `--amendment` | Hash256 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfGotMajority` (65536), `tfInnerBatchTxn` (1073741824), `tfLostMajority` (131072)

## EscrowCancel — `xrpl tx new escrow-cancel`

| flag | type | required |
|---|---|---|
| `--owner` | AccountID | yes |
| `--offer-sequence` | UInt32 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## EscrowCreate — `xrpl tx new escrow-create`

| flag | type | required |
|---|---|---|
| `--destination` | AccountID | yes |
| `--amount` | Amount | yes |
| `--condition` | Blob |  |
| `--cancel-after` | UInt32 |  |
| `--finish-after` | UInt32 |  |
| `--destination-tag` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## EscrowFinish — `xrpl tx new escrow-finish`

| flag | type | required |
|---|---|---|
| `--owner` | AccountID | yes |
| `--offer-sequence` | UInt32 | yes |
| `--fulfillment` | Blob |  |
| `--condition` | Blob |  |
| `--credential-ids` | Vector256 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## LedgerStateFix — `xrpl tx new ledger-state-fix`

| flag | type | required |
|---|---|---|
| `--ledger-fix-type` | UInt16 | yes |
| `--owner` | AccountID |  |
| `--book-directory` | Hash256 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## LoanBrokerCoverClawback — `xrpl tx new loan-broker-cover-clawback`

| flag | type | required |
|---|---|---|
| `--loan-broker-id` | Hash256 |  |
| `--amount` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## LoanBrokerCoverDeposit — `xrpl tx new loan-broker-cover-deposit`

| flag | type | required |
|---|---|---|
| `--loan-broker-id` | Hash256 | yes |
| `--amount` | Amount | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## LoanBrokerCoverWithdraw — `xrpl tx new loan-broker-cover-withdraw`

| flag | type | required |
|---|---|---|
| `--loan-broker-id` | Hash256 | yes |
| `--amount` | Amount | yes |
| `--destination` | AccountID |  |
| `--destination-tag` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## LoanBrokerDelete — `xrpl tx new loan-broker-delete`

| flag | type | required |
|---|---|---|
| `--loan-broker-id` | Hash256 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## LoanBrokerSet — `xrpl tx new loan-broker-set`

| flag | type | required |
|---|---|---|
| `--vault-id` | Hash256 | yes |
| `--loan-broker-id` | Hash256 |  |
| `--data` | Blob |  |
| `--management-fee-rate` | UInt16 |  |
| `--debt-maximum` | Number |  |
| `--cover-rate-minimum` | UInt32 |  |
| `--cover-rate-liquidation` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## LoanDelete — `xrpl tx new loan-delete`

| flag | type | required |
|---|---|---|
| `--loan-id` | Hash256 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## LoanManage — `xrpl tx new loan-manage`

| flag | type | required |
|---|---|---|
| `--loan-id` | Hash256 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfLoanDefault` (65536), `tfLoanImpair` (131072), `tfLoanUnimpair` (262144)

## LoanPay — `xrpl tx new loan-pay`

| flag | type | required |
|---|---|---|
| `--loan-id` | Hash256 | yes |
| `--amount` | Amount | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfLoanFullPayment` (131072), `tfLoanLatePayment` (262144), `tfLoanOverpayment` (65536)

## LoanSet — `xrpl tx new loan-set`

| flag | type | required |
|---|---|---|
| `--loan-broker-id` | Hash256 | yes |
| `--data` | Blob |  |
| `--counterparty` | AccountID |  |
| `--counterparty-signature` | STObject |  |
| `--loan-origination-fee` | Number |  |
| `--loan-service-fee` | Number |  |
| `--late-payment-fee` | Number |  |
| `--close-payment-fee` | Number |  |
| `--overpayment-fee` | UInt32 |  |
| `--interest-rate` | UInt32 |  |
| `--late-interest-rate` | UInt32 |  |
| `--close-interest-rate` | UInt32 |  |
| `--overpayment-interest-rate` | UInt32 |  |
| `--principal-requested` | Number | yes |
| `--payment-total` | UInt32 |  |
| `--payment-interval` | UInt32 |  |
| `--grace-period` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfLoanOverpayment` (65536)

## MPTokenAuthorize — `xrpl tx new mptoken-authorize`

| flag | type | required |
|---|---|---|
| `--mptoken-issuance-id` | Hash192 | yes |
| `--holder` | AccountID |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfMPTUnauthorize` (1)

## MPTokenIssuanceCreate — `xrpl tx new mptoken-issuance-create`

| flag | type | required |
|---|---|---|
| `--asset-scale` | UInt8 |  |
| `--transfer-fee` | UInt16 |  |
| `--maximum-amount` | UInt64 |  |
| `--mptoken-metadata` | Blob |  |
| `--domain-id` | Hash256 |  |
| `--immutable-flags` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfMPTCanClawback` (64), `tfMPTCanEscrow` (8), `tfMPTCanHoldConfidentialBalance` (128), `tfMPTCanLock` (2), `tfMPTCanTrade` (16), `tfMPTCanTransfer` (32), `tfMPTRequireAuth` (4)

## MPTokenIssuanceDestroy — `xrpl tx new mptoken-issuance-destroy`

| flag | type | required |
|---|---|---|
| `--mptoken-issuance-id` | Hash192 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## MPTokenIssuanceSet — `xrpl tx new mptoken-issuance-set`

| flag | type | required |
|---|---|---|
| `--mptoken-issuance-id` | Hash192 | yes |
| `--holder` | AccountID |  |
| `--domain-id` | Hash256 |  |
| `--mptoken-metadata` | Blob |  |
| `--transfer-fee` | UInt16 |  |
| `--immutable-flags` | UInt32 |  |
| `--issuer-encryption-key` | Blob |  |
| `--auditor-encryption-key` | Blob |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfMPTLock` (1), `tfMPTSetCanClawback` (128), `tfMPTSetCanEscrow` (16), `tfMPTSetCanHoldConfidentialBalance` (256), `tfMPTSetCanLock` (4), `tfMPTSetCanTrade` (32), `tfMPTSetCanTransfer` (64), `tfMPTSetRequireAuth` (8), `tfMPTUnlock` (2)

## NFTokenAcceptOffer — `xrpl tx new nftoken-accept-offer`

| flag | type | required |
|---|---|---|
| `--nftoken-buy-offer` | Hash256 |  |
| `--nftoken-sell-offer` | Hash256 |  |
| `--nftoken-broker-fee` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## NFTokenBurn — `xrpl tx new nftoken-burn`

| flag | type | required |
|---|---|---|
| `--nftoken-id` | Hash256 | yes |
| `--owner` | AccountID |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## NFTokenCancelOffer — `xrpl tx new nftoken-cancel-offer`

| flag | type | required |
|---|---|---|
| `--nftoken-offers` | Vector256 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## NFTokenCreateOffer — `xrpl tx new nftoken-create-offer`

| flag | type | required |
|---|---|---|
| `--nftoken-id` | Hash256 | yes |
| `--amount` | Amount | yes |
| `--destination` | AccountID |  |
| `--owner` | AccountID |  |
| `--expiration` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfSellNFToken` (1)

## NFTokenMint — `xrpl tx new nftoken-mint`

| flag | type | required |
|---|---|---|
| `--nftoken-taxon` | UInt32 | yes |
| `--transfer-fee` | UInt16 |  |
| `--issuer` | AccountID |  |
| `--uri` | Blob |  |
| `--amount` | Amount |  |
| `--destination` | AccountID |  |
| `--expiration` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfBurnable` (1), `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfMutable` (16), `tfOnlyXRP` (2), `tfTransferable` (8)

## NFTokenModify — `xrpl tx new nftoken-modify`

| flag | type | required |
|---|---|---|
| `--nftoken-id` | Hash256 | yes |
| `--owner` | AccountID |  |
| `--uri` | Blob |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## OfferCancel — `xrpl tx new offer-cancel`

| flag | type | required |
|---|---|---|
| `--offer-sequence` | UInt32 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## OfferCreate — `xrpl tx new offer-create`

| flag | type | required |
|---|---|---|
| `--taker-pays` | Amount | yes |
| `--taker-gets` | Amount | yes |
| `--expiration` | UInt32 |  |
| `--offer-sequence` | UInt32 |  |
| `--domain-id` | Hash256 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFillOrKill` (262144), `tfFullyCanonicalSig` (2147483648), `tfHybrid` (1048576), `tfImmediateOrCancel` (131072), `tfInnerBatchTxn` (1073741824), `tfPassive` (65536), `tfSell` (524288)

## OracleDelete — `xrpl tx new oracle-delete`

| flag | type | required |
|---|---|---|
| `--oracle-document-id` | UInt32 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## OracleSet — `xrpl tx new oracle-set`

| flag | type | required |
|---|---|---|
| `--oracle-document-id` | UInt32 | yes |
| `--provider` | Blob |  |
| `--uri` | Blob |  |
| `--asset-class` | Blob |  |
| `--last-update-time` | UInt32 | yes |
| `--price-data-series` | STArray | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## Payment — `xrpl tx new payment`

| flag | type | required |
|---|---|---|
| `--destination` | AccountID | yes |
| `--amount` | Amount | yes |
| `--send-max` | Amount |  |
| `--paths` | PathSet |  |
| `--invoice-id` | Hash256 |  |
| `--destination-tag` | UInt32 |  |
| `--deliver-min` | Amount |  |
| `--credential-ids` | Vector256 |  |
| `--domain-id` | Hash256 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfLimitQuality` (262144), `tfNoRippleDirect` (65536), `tfPartialPayment` (131072), `tfSponsorCreatedAccount` (524288)

## PaymentChannelClaim — `xrpl tx new payment-channel-claim`

| flag | type | required |
|---|---|---|
| `--channel` | Hash256 | yes |
| `--amount` | Amount |  |
| `--balance` | Amount |  |
| `--signature` | Blob |  |
| `--public-key` | Blob |  |
| `--credential-ids` | Vector256 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfClose` (131072), `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfRenew` (65536)

## PaymentChannelCreate — `xrpl tx new payment-channel-create`

| flag | type | required |
|---|---|---|
| `--destination` | AccountID | yes |
| `--amount` | Amount | yes |
| `--settle-delay` | UInt32 | yes |
| `--public-key` | Blob | yes |
| `--cancel-after` | UInt32 |  |
| `--destination-tag` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## PaymentChannelFund — `xrpl tx new payment-channel-fund`

| flag | type | required |
|---|---|---|
| `--channel` | Hash256 | yes |
| `--amount` | Amount | yes |
| `--expiration` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## PermissionedDomainDelete — `xrpl tx new permissioned-domain-delete`

| flag | type | required |
|---|---|---|
| `--domain-id` | Hash256 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## PermissionedDomainSet — `xrpl tx new permissioned-domain-set`

| flag | type | required |
|---|---|---|
| `--domain-id` | Hash256 |  |
| `--accepted-credentials` | STArray | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## SetFee — `xrpl tx new set-fee`

| flag | type | required |
|---|---|---|
| `--ledger-sequence` | UInt32 |  |
| `--base-fee` | UInt64 |  |
| `--reference-fee-units` | UInt32 |  |
| `--reserve-base` | UInt32 |  |
| `--reserve-increment` | UInt32 |  |
| `--base-fee-drops` | Amount |  |
| `--reserve-base-drops` | Amount |  |
| `--reserve-increment-drops` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## SetRegularKey — `xrpl tx new set-regular-key`

| flag | type | required |
|---|---|---|
| `--regular-key` | AccountID |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## SignerListSet — `xrpl tx new signer-list-set`

| flag | type | required |
|---|---|---|
| `--signer-quorum` | UInt32 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## SponsorshipSet — `xrpl tx new sponsorship-set`

| flag | type | required |
|---|---|---|
| `--counterparty-sponsor` | AccountID |  |
| `--sponsee` | AccountID |  |
| `--fee-amount` | Amount |  |
| `--max-fee` | Amount |  |
| `--remaining-owner-count` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfDeleteObject` (1048576), `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfSponsorshipClearRequireSignForFee` (131072), `tfSponsorshipClearRequireSignForReserve` (524288), `tfSponsorshipSetRequireSignForFee` (65536), `tfSponsorshipSetRequireSignForReserve` (262144)

## SponsorshipTransfer — `xrpl tx new sponsorship-transfer`

| flag | type | required |
|---|---|---|
| `--object-id` | Hash256 |  |
| `--sponsee` | AccountID |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfSponsorshipCreate` (131072), `tfSponsorshipEnd` (65536), `tfSponsorshipReassign` (262144)

## TicketCreate — `xrpl tx new ticket-create`

| flag | type | required |
|---|---|---|
| `--ticket-count` | UInt32 | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## TrustSet — `xrpl tx new trust-set`

| flag | type | required |
|---|---|---|
| `--limit-amount` | Amount |  |
| `--quality-in` | UInt32 |  |
| `--quality-out` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfClearDeepFreeze` (8388608), `tfClearFreeze` (2097152), `tfClearNoRipple` (262144), `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfSetDeepFreeze` (4194304), `tfSetFreeze` (1048576), `tfSetNoRipple` (131072), `tfSetfAuth` (65536)

## UNLModify — `xrpl tx new unlmodify`

| flag | type | required |
|---|---|---|
| `--unlmodify-disabling` | UInt8 | yes |
| `--ledger-sequence` | UInt32 | yes |
| `--unlmodify-validator` | Blob | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## VaultClawback — `xrpl tx new vault-clawback`

| flag | type | required |
|---|---|---|
| `--vault-id` | Hash256 | yes |
| `--holder` | AccountID | yes |
| `--amount` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## VaultCreate — `xrpl tx new vault-create`

| flag | type | required |
|---|---|---|
| `--asset` | Issue | yes |
| `--assets-maximum` | Number |  |
| `--mptoken-metadata` | Blob |  |
| `--domain-id` | Hash256 |  |
| `--withdrawal-policy` | UInt8 |  |
| `--data` | Blob |  |
| `--scale` | UInt8 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824), `tfVaultPrivate` (65536), `tfVaultShareNonTransferable` (131072)

## VaultDelete — `xrpl tx new vault-delete`

| flag | type | required |
|---|---|---|
| `--vault-id` | Hash256 | yes |
| `--memo-data` | Blob |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## VaultDeposit — `xrpl tx new vault-deposit`

| flag | type | required |
|---|---|---|
| `--vault-id` | Hash256 | yes |
| `--amount` | Amount | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## VaultSet — `xrpl tx new vault-set`

| flag | type | required |
|---|---|---|
| `--vault-id` | Hash256 | yes |
| `--assets-maximum` | Number |  |
| `--domain-id` | Hash256 |  |
| `--data` | Blob |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## VaultWithdraw — `xrpl tx new vault-withdraw`

| flag | type | required |
|---|---|---|
| `--vault-id` | Hash256 | yes |
| `--amount` | Amount | yes |
| `--destination` | AccountID |  |
| `--destination-tag` | UInt32 |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## XChainAccountCreateCommit — `xrpl tx new xchain-account-create-commit`

| flag | type | required |
|---|---|---|
| `--xchain-bridge` | XChainBridge | yes |
| `--destination` | AccountID | yes |
| `--amount` | Amount | yes |
| `--signature-reward` | Amount | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## XChainAddAccountCreateAttestation — `xrpl tx new xchain-add-account-create-attestation`

| flag | type | required |
|---|---|---|
| `--xchain-bridge` | XChainBridge | yes |
| `--attestation-signer-account` | AccountID | yes |
| `--public-key` | Blob | yes |
| `--signature` | Blob | yes |
| `--other-chain-source` | AccountID | yes |
| `--amount` | Amount | yes |
| `--attestation-reward-account` | AccountID | yes |
| `--was-locking-chain-send` | UInt8 | yes |
| `--xchain-account-create-count` | UInt64 | yes |
| `--destination` | AccountID | yes |
| `--signature-reward` | Amount | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## XChainAddClaimAttestation — `xrpl tx new xchain-add-claim-attestation`

| flag | type | required |
|---|---|---|
| `--xchain-bridge` | XChainBridge | yes |
| `--attestation-signer-account` | AccountID | yes |
| `--public-key` | Blob | yes |
| `--signature` | Blob | yes |
| `--other-chain-source` | AccountID | yes |
| `--amount` | Amount | yes |
| `--attestation-reward-account` | AccountID | yes |
| `--was-locking-chain-send` | UInt8 | yes |
| `--xchain-claim-id` | UInt64 | yes |
| `--destination` | AccountID |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## XChainClaim — `xrpl tx new xchain-claim`

| flag | type | required |
|---|---|---|
| `--xchain-bridge` | XChainBridge | yes |
| `--xchain-claim-id` | UInt64 | yes |
| `--destination` | AccountID | yes |
| `--destination-tag` | UInt32 |  |
| `--amount` | Amount | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## XChainCommit — `xrpl tx new xchain-commit`

| flag | type | required |
|---|---|---|
| `--xchain-bridge` | XChainBridge | yes |
| `--xchain-claim-id` | UInt64 | yes |
| `--amount` | Amount | yes |
| `--other-chain-destination` | AccountID |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## XChainCreateBridge — `xrpl tx new xchain-create-bridge`

| flag | type | required |
|---|---|---|
| `--xchain-bridge` | XChainBridge | yes |
| `--signature-reward` | Amount | yes |
| `--min-account-create-amount` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## XChainCreateClaimID — `xrpl tx new xchain-create-claim-id`

| flag | type | required |
|---|---|---|
| `--xchain-bridge` | XChainBridge | yes |
| `--signature-reward` | Amount | yes |
| `--other-chain-source` | AccountID | yes |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

## XChainModifyBridge — `xrpl tx new xchain-modify-bridge`

| flag | type | required |
|---|---|---|
| `--xchain-bridge` | XChainBridge | yes |
| `--signature-reward` | Amount |  |
| `--min-account-create-amount` | Amount |  |
| `--source-tag` | UInt32 |  |
| `--account` | AccountID |  |
| `--sequence` | UInt32 |  |
| `--last-ledger-sequence` | UInt32 |  |
| `--account-txn-id` | Hash256 |  |
| `--fee` | Amount |  |
| `--operation-limit` | UInt32 |  |
| `--ticket-sequence` | UInt32 |  |
| `--network-id` | UInt32 |  |
| `--delegate` | AccountID |  |
| `--sponsor` | AccountID |  |
| `--sponsor-flags` | UInt32 |  |

Flags: `tfClearAccountCreateAmount` (65536), `tfFullyCanonicalSig` (2147483648), `tfInnerBatchTxn` (1073741824)

