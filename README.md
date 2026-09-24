# xrpl-rust ![Downloads](https://img.shields.io/crates/d/xrpl-rust)

[![latest]][crates.io] [![deps_status]][deps] [![audit_status]][audit] [![unit_status]][unit]

[latest]: https://img.shields.io/crates/v/xrpl-rust.svg
[crates.io]: https://crates.io/crates/xrpl-rust
[docs_status]: https://docs.rs/xrpl-rust/badge.svg
[docs]: https://docs.rs/xrpl-rust/latest/xrpl/
[deps_status]: https://deps.rs/repo/github/589labs/xrpl-rust/status.svg
[deps]: https://deps.rs/repo/github/589labs/xrpl-rust
[audit_status]: https://github.com/589labs/xrpl-rust/actions/workflows/audit_test.yml/badge.svg
[audit]: https://github.com/589labs/xrpl-rust/actions/workflows/audit_test.yml
[rustc]: https://img.shields.io/badge/rust-1.51.0%2B-orange.svg
[rust]: https://blog.rust-lang.org/2021/03/25/Rust-1.51.0.html
[unit_status]: https://github.com/589labs/xrpl-rust/actions/workflows/unit_test.yml/badge.svg
[unit]: https://github.com/589labs/xrpl-rust/actions/workflows/unit_test.yml
[contributors]: https://github.com/589labs/xrpl-rust/graphs/contributors
[contributors_status]: https://img.shields.io/github/contributors/589labs/xrpl-rust.svg
[license]: https://opensource.org/licenses/ISC
[license_status]: https://img.shields.io/badge/License-ISC-blue.svg

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="/assets/xrpl-rust_white.png">
  <img alt="" src="/assets/xrpl-rust_black.png">
</picture>

> [!WARNING]
> This repository is under active development.

A Rust library to interact with the XRPL.
Based off of the [xrpl-py](https://github.com/XRPLF/xrpl-py) library.

A pure Rust implementation for interacting with the XRP Ledger. The xrpl-rust
crate simplifies the hardest parts of XRP Ledger interaction including
serialization and transaction signing while providing idiomatic Rust
functionality for XRP Ledger transactions and core server API (rippled)
objects.

Interactions with this crate occur using data structures from this crate or
core [alloc](https://doc.rust-lang.org/alloc) types with the exception of
serde for JSON handling and indexmap for dictionaries. The goal is to ensure
this library can be used on devices without the ability to use a
[std](https://doc.rust-lang.org/std) environment.

# Table of Contents

- [Installation](#installation)
- [Documentation](#documentation)
- [Quickstart](#quickstart)
- [Feature Flags](#feature-flags)
- [`no_std` Support](#no_std)
- [Command Line Interface](#command-line-interface)
  - [The tx pipeline](#the-tx-pipeline)
  - [Accounts, keys, and signers](#accounts-keys-and-signers)
  - [Command groups](#command-groups)
  - [A worked example](#a-worked-example)
- [Library Usage](#library-usage)
- [Contributing](#contributing)
- [License](#license)

# 🛠 Installation [![rustc]][rust]

To install, add the following to your project's `Cargo.toml`:

```toml
[dependencies.xrpl]
version = "1.2.0"
```

# Documentation [![docs_status]][docs]

Documentation is available [here](https://docs.rs/xrpl-rust).

# Quickstart

## Basic Wallet Operations

```rust
use xrpl::wallet::Wallet;

// Generate a new wallet
let wallet = Wallet::create(None)?;
println!("Address: {}", wallet.classic_address);

// Create wallet from seed
let wallet = Wallet::from_seed("sEdV19BLfeQeKdEXyYA4NhjPJe6XBfG", None, false)?;
```

## Making Requests

```rust
use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_info::AccountInfo;

let client = XRPLSyncClient::new("https://xrplcluster.com/")?;
let req = AccountInfo::builder("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh").build();
let response = client.request(req.into())?;
```

# Feature Flags

## Default Features

- `std` - Standard library support
- `core` - Core XRPL functionality
- `models` - XRPL data models
- `wallet` - Wallet operations
- `utils` - Utility functions
- `websocket` - WebSocket client
- `json-rpc` - JSON-RPC client
- `helpers` - Helper functions (requires runtime)
- `tokio-rt` - Tokio async runtime

## Optional Features

- `cli` - Command line interface
- `embassy-rt` - Embassy async runtime (for no_std)
- `serde` - Serialization support

## Runtime Requirements

When using `helpers`, you must specify a runtime:

- `tokio-rt` - For std environments
- `embassy-rt` - For no_std environments

## #![no_std]

This library aims to be `#![no_std]` compliant.

## `no_std` Usage

```toml
[dependencies.xrpl]
version = "*"
default-features = false
features = ["core", "models", "wallet", "utils", "websocket", "json-rpc", "helpers", "embassy-rt"]
```

# Command Line Interface

`xrpl-cli` is a separate crate, so depending on `xrpl-rust` does not pull `clap` into your build.

**[`xrpl-cli/README.md`](xrpl-cli/README.md) is the reference.** What follows is
an orientation; the command surface changes, and two copies of it would drift.

```bash
cargo install --path xrpl-cli
xrpl --help
```

## The `tx` pipeline

Every stage reads one machine artifact on stdin and writes one on stdout, so
they compose. Human-readable output goes to stderr and is silenced by `-q`.

```bash
xrpl tx new Payment \
      --account rISSUER… --destination rDEST… --amount 10000000 \
  | xrpl tx autofill --network testnet \
  | xrpl tx sign --sign-with issuer \
  | xrpl tx submit --wait --network testnet
```

`xrpl tx new` covers every transaction type the bundled definitions carry, with
a flag per field. `autofill` and `submit` are the only stages that reach a node,
and neither has a default network — a pipeline stage that picks one on your
behalf can pick mainnet.

## Accounts, keys, and signers

Three separate things, on purpose. An **account** is an address and what is true
of it offline; a **key** is a public key and a pointer to where its secret
lives; a **signer** is resolved at runtime and never persisted.

```bash
xrpl key generate issuer --show-secret      # the 16-byte seed IS the backup
xrpl account add issuer --key issuer --network-id 0
xrpl account doctor issuer --ledger         # local record vs. the ledger
```

No address is spelled out: an account's address derives from its original master
public key, so `--key` already carries one. `--address` is still there for the
cases where the two legitimately differ — a regular key, or a key that is only a
member of someone else's signer list — and the one key an account has is the key
that signs it, so `--default-signer` is for an account with several.

Seeds are encrypted at rest under a passphrase (age, scrypt) and the record
points at the blob rather than holding it. Nothing this CLI writes contains key
material in the clear. The honest claim is **encrypted at rest, plaintext in
process memory at signing time** — see `xrpl-cli/src/store/secret.rs` for what
that does and does not defend against.

For a signature without enrolling anything, `--seed-file` reads a file you own
and the CLI never writes one:

```bash
xrpl tx sign --seed-file <(op read op://vault/issuer/seed)
```

## Command groups

| Group | What it does |
| --- | --- |
| `tx` | Build, edit, autofill, sign, multisign, merge, submit, hash, decode |
| `account` | Local records (`add`, `ls`, `show`, `rm`, `use`, `doctor`) **and** ledger queries (`info`, `tx`/`history`, `lines`, `objects`, `channels`, `currencies`, `nfts`) |
| `key` | `add`, `generate`, `export`, `ls`, `show`, `rm` |
| `server` / `ledger` | `fee`, `info`, `subscribe`, `data` |
| `rpc` | Any rippled method, by name |

`account show` reads one local file; `account info` calls the node. Each verb's
help says which side it is on.

## A worked example

[`xrpl-cli/demo/token-ceremony.sh`](xrpl-cli/demo/token-ceremony.sh) issues an
MPT from an account whose master key is disabled and which is controlled 2-of-3,
collecting the quorum both serially and in parallel — using only `xrpl`
commands. It is also the CI acceptance gate, so it cannot drift.


# Library Usage

## Basic Wallet Operations

```rust
use xrpl::wallet::Wallet;

// Generate a new wallet
let wallet = Wallet::create(None)?;
println!("Address: {}", wallet.classic_address);
println!("Seed: {}", wallet.seed);

// Create wallet from seed
let seed = "sEdV19BLfeQeKdEXyYA4NhjPJe6XBfG";
let wallet = Wallet::from_seed(seed, None, false)?;
println!("Classic Address: {}", wallet.classic_address);
```

### Constructing Models (Builders)

Every request and transaction model is constructed through a [`bon`](https://bon-rs.com) builder instead of a positional constructor:

- `Type::builder(<subject>)` starts the builder. The subject is the field the model is *about* — the sending `account` for transactions and account-scoped requests, `taker_gets` for `BookOffers`, the `command` for `GenericRequest`.
- Optional fields are named setters that take the value directly — `.limit(10)`, `.ledger_index(LedgerIndex::Validated)`. Leave a setter off to leave the field unset; there are no `None` placeholders.
- String-ish and amount-ish setters accept anything convertible, so `&str` works where `Cow<'a, str>` or `XRPAmount` is expected: `.fee("12")`, `.destination("rReceiver456")`.
- When you already hold an `Option<T>`, use the `maybe_` form of the setter: `.maybe_ledger_hash(hash_opt)`.
- `.build()` finishes and returns the model.

```rust
use xrpl::models::requests::account_tx::AccountTx;

let req = AccountTx::builder("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh")
    .ledger_index_min(1)
    .ledger_index_max(99999)
    .forward(true)
    .limit(25)
    .build();
```

Setters are order-independent, and adding a new optional field to a model no longer breaks existing call sites.

### Making API Requests

```rust
use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::{
    account_info::AccountInfo,
    account_lines::AccountLines,
    book_offers::BookOffers,
    ledger::Ledger,
};
use xrpl::models::{LedgerIndex, Currency};

let client = XRPLSyncClient::new("https://xrplcluster.com/")?;

// Get account information
let account_info_req = AccountInfo::builder("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh").build();
let response = client.request(account_info_req.into())?;

// Get account trust lines — only the fields you care about
let account_lines_req = AccountLines::builder("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh")
    .limit(10)
    .build();
let lines_response = client.request(account_lines_req.into())?;

// Get order book offers
let taker_gets = Currency::xrp();
let taker_pays = Currency::issued("USD", "rvYAfWj5gh67oV6fW32ZzP3Aw4Eubs59B");
let book_offers_req = BookOffers::builder(taker_gets)
    .taker_pays(taker_pays)
    .limit(5)
    .build();
let offers_response = client.request(book_offers_req.into())?;
```

### Working with Transactions (New Builder Pattern)

```rust
use xrpl::models::transactions::{Payment, AccountSet, AccountDelete, CommonFields};
use xrpl::models::{Amount, TransactionType};
use xrpl::wallet::Wallet;
use xrpl::clients::XRPLSyncClient;

// Create a simple XRP payment using the new builder pattern
let payment = Payment {
    common_fields: CommonFields {
        account: "rSenderAddress123".into(),
        transaction_type: TransactionType::Payment,
        ..Default::default()
    },
    amount: Amount::xrp_amount("1000000"), // 1 XRP in drops
    destination: "rDestinationAddress456".into(),
    ..Default::default()
}
.with_fee("12")
.with_sequence(100)
.with_destination_tag(12345)
.with_memo(Memo {
    memo_data: Some("payment memo".into()),
    memo_format: None,
    memo_type: Some("text".into()),
});

// Create a cross-currency payment with path finding
let cross_currency_payment = Payment {
    common_fields: CommonFields {
        account: "rSenderAddress123".into(),
        transaction_type: TransactionType::Payment,
        ..Default::default()
    },
    amount: Amount::IssuedCurrencyAmount(IssuedCurrencyAmount::new(
        "USD".into(),
        "rUSDIssuer789".into(),
        "100".into(),
    )),
    destination: "rDestinationAddress456".into(),
    ..Default::default()
}
.with_send_max(Amount::xrp_amount("110000000")) // Max 110 XRP
.with_flag(PaymentFlag::TfPartialPayment)
.with_fee("12")
.with_sequence(101);

// Set up an account with deposit authorization
let account_setup = AccountSet {
    common_fields: CommonFields {
        account: "rAccountToSetup123".into(),
        transaction_type: TransactionType::AccountSet,
        ..Default::default()
    },
    ..Default::default()
}
.with_set_flag(AccountSetFlag::AsfDepositAuth)
.with_transfer_rate(1020000000) // 2% transfer fee
.with_fee("12")
.with_sequence(50);

// Delete an account
let account_deletion = AccountDelete {
    common_fields: CommonFields {
        account: "rAccountToDelete456".into(),
        transaction_type: TransactionType::AccountDelete,
        ..Default::default()
    },
    destination: "rDestinationAccount789".into(),
    ..Default::default()
}
.with_destination_tag(98765)
.with_fee("2000000") // 2 XRP minimum fee for account deletion
.with_sequence(200)
.with_memo(Memo {
    memo_data: Some("closing account".into()),
    memo_format: None,
    memo_type: Some("text".into()),
});

// Sign and submit transactions
let wallet = Wallet::from_seed("sEdV19BLfeQeKdEXyYA4NhjPJe6XBfG", None, false)?;
let client = XRPLSyncClient::new("https://s.altnet.rippletest.net:51234")?;

let signed_payment = wallet.sign(&payment.into(), Some(true))?;
let submit_response = client.submit(signed_payment)?;
```

### Working with AMM Transactions

```rust
use xrpl::models::transactions::{AMMCreate, AMMBid, AMMDelete};
use xrpl::models::{Amount, Currency, IssuedCurrencyAmount};
use xrpl::models::currency::XRP;

// Create an AMM pool
let amm_create = AMMCreate {
    common_fields: CommonFields {
        account: "rAMMCreator123".into(),
        transaction_type: TransactionType::AMMCreate,
        ..Default::default()
    },
    amount: Amount::XRPAmount(XRPAmount::from("50000000")), // 50 XRP
    amount2: Amount::IssuedCurrencyAmount(IssuedCurrencyAmount::new(
        "USD".into(),
        "rUSDIssuer456".into(),
        "50".into(), // 50 USD
    )),
    trading_fee: 100, // 0.1% trading fee
}
.with_fee("12")
.with_sequence(100)
.with_memo(Memo {
    memo_data: Some("creating XRP-USD AMM".into()),
    memo_format: None,
    memo_type: Some("text".into()),
});

// Bid on AMM auction slot
let amm_bid = AMMBid {
    common_fields: CommonFields {
        account: "rBidder789".into(),
        transaction_type: TransactionType::AMMBid,
        ..Default::default()
    },
    asset: Currency::XRP(XRP::new()),
    asset2: Currency::IssuedCurrency(IssuedCurrency::new(
        "USD".into(),
        "rUSDIssuer456".into(),
    )),
    ..Default::default()
}
.with_bid_min(IssuedCurrencyAmount::new(
    "039C99CD9AB0B70B32ECDA51EAAE471625608EA2".into(),
    "rLPTokenIssuer".into(),
    "100".into(),
))
.with_bid_max(IssuedCurrencyAmount::new(
    "039C99CD9AB0B70B32ECDA51EAAE471625608EA2".into(),
    "rLPTokenIssuer".into(),
    "200".into(),
))
.with_fee("15")
.with_sequence(200);

// Delete empty AMM
let amm_delete = AMMDelete {
    common_fields: CommonFields {
        account: "rAMMDeleter111".into(),
        transaction_type: TransactionType::AMMDelete,
        ..Default::default()
    },
    asset: Currency::XRP(XRP::new()),
    asset2: Currency::IssuedCurrency(IssuedCurrency::new(
        "USD".into(),
        "rUSDIssuer456".into(),
    )),
    ..Default::default()
}
.with_fee("12")
.with_sequence(300);
```

### Address Conversion

```rust
use xrpl::core::addresscodec::{
    classic_address_to_xaddress,
    xaddress_to_classic_address,
    is_valid_classic_address,
};

// Convert classic address to X-address
let classic_address = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";
let xaddress = classic_address_to_xaddress(classic_address, None, false)?;
println!("X-Address: {}", xaddress);

// Convert X-address back to classic address
let (address, tag, is_test) = xaddress_to_classic_address(&xaddress)?;
println!("Classic Address: {}, Tag: {:?}, Test Network: {}", address, tag, is_test);

// Validate addresses
let is_valid = is_valid_classic_address(classic_address, None);
println!("Address is valid: {}", is_valid);
```

### Working with NFTs

```rust
use xrpl::models::transactions::{NFTokenMint, NFTokenCreateOffer, NFTokenAcceptOffer};
use xrpl::models::Amount;

// Mint an NFT
let nft_mint = NFTokenMint {
    common_fields: CommonFields {
        account: "rNFTMinter123".into(),
        transaction_type: TransactionType::NFTokenMint,
        ..Default::default()
    },
    nftoken_taxon: 0,
    ..Default::default()
}
.with_fee("12")
.with_sequence(100)
.with_memo(Memo {
    memo_data: Some("minting unique NFT".into()),
    memo_format: None,
    memo_type: Some("text".into()),
});

// Create an NFT sell offer
let nft_sell_offer = NFTokenCreateOffer {
    common_fields: CommonFields {
        account: "rNFTSeller456".into(),
        transaction_type: TransactionType::NFTokenCreateOffer,
        ..Default::default()
    },
    nftoken_id: "000B013A95F14B0E44F78A264E41713C64B5F89242540EE208C3098E00000D65".into(),
    ..Default::default()
}
.with_amount(Amount::xrp_amount("1000000")) // 1 XRP
.with_fee("12")
.with_sequence(200);
```

### Binary Codec Usage

```rust
use xrpl::core::binarycodec::{encode, decode};
use serde_json::json;

// Encode transaction to binary
let tx_json = json!({
    "TransactionType": "Payment",
    "Account": "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh",
    "Destination": "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe",
    "Amount": "1000000",
    "Fee": "12",
    "Sequence": 1
});

let encoded = encode(&tx_json, Some(true))?; // true for signing
println!("Encoded transaction: {}", encoded);

// Decode binary back to JSON
let decoded = decode(&encoded)?;
println!("Decoded transaction: {}", serde_json::to_string_pretty(&decoded)?);
```

### Utility Functions

```rust
use xrpl::utils::{
    xrp_to_drops, drops_to_xrp,
    posix_to_ripple_time, ripple_time_to_posix,
};

// XRP conversion
let xrp_amount = "1.5";
let drops = xrp_to_drops(xrp_amount)?;
println!("1.5 XRP = {} drops", drops);

let xrp_back = drops_to_xrp(&drops)?;
println!("{} drops = {} XRP", drops, xrp_back);

// Time conversion
let posix_time = 1660187459;
let ripple_time = posix_to_ripple_time(posix_time)?;
println!("POSIX {} = Ripple time {}", posix_time, ripple_time);

let posix_back = ripple_time_to_posix(ripple_time)?;
println!("Ripple time {} = POSIX {}", ripple_time, posix_back);
```

### Error Handling

```rust
use xrpl::models::exceptions::XRPLModelException;
use xrpl::core::exceptions::XRPLCoreException;
use xrpl::wallet::exceptions::XRPLWalletException;

// Proper error handling example
match Wallet::from_seed("invalid_seed", None, false) {
    Ok(wallet) => println!("Wallet created: {}", wallet.classic_address),
    Err(XRPLWalletException::InvalidSeed(msg)) => {
        eprintln!("Invalid seed provided: {}", msg);
    },
    Err(e) => eprintln!("Other wallet error: {:?}", e),
}

// Transaction validation
let payment = Payment {
    common_fields: CommonFields {
        account: "rSender123".into(),
        transaction_type: TransactionType::Payment,
        ..Default::default()
    },
    amount: Amount::xrp_amount("1000000"),
    destination: "rReceiver456".into(),
    ..Default::default()
}
.with_fee("12")
.with_sequence(100);

match payment.validate() {
    Ok(_) => println!("Transaction is valid"),
    Err(e) => eprintln!("Transaction validation failed: {}", e),
}
```

# Contributing [![contributors_status]][contributors]

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## Development Setup

```bash
# Clone the repository
git clone https://github.com/sephynox/xrpl-rust.git
cd xrpl-rust

# Run tests
cargo test

# Run CLI tests
cargo test -p xrpl-cli

# Run the CLI's integration tests and acceptance gate against a standalone node
docker run -d -p 5005:5005 -p 6006:6006 \
  -v "$PWD/.ci-config/:/etc/xrpld/" --name xrpld rippleci/xrpld:develop --standalone
cargo test -p xrpl-cli --features integration -- --test-threads=1
XRPL_DATA_DIR=$(mktemp -d) XRPL_NETWORK=local ./xrpl-cli/demo/token-ceremony.sh all

# Build with all features
cargo build --all-features
```

# License [![license_status]][license]

This project is licensed under the [ISC License](LICENSE).
