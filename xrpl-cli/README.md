# xrpl-cli

Command line interface for the XRP Ledger, built on [`xrpl-rust`](https://crates.io/crates/xrpl-rust).

```bash
cargo install xrpl-cli
xrpl --help
```

## Choosing a network

Every command that reaches a node takes `--url` or `--network`:

```bash
xrpl account info --address rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh --network mainnet
xrpl server info --network local          # http://127.0.0.1:5005
xrpl server subscribe --network local     # ws://127.0.0.1:6006
```

`--network` accepts `mainnet`, `testnet`, `devnet` and `local`. `--url` wins when both are given.

## Layout

One module per command, grouped by domain:

```
src/commands/
  global.rs            --url / --network, shared defaults
  wallet/              generate, from-seed, faucet, validate
  account/             info, tx, objects, channels, currencies, lines, nfts, set-flag, clear-flag
  transaction/         sign, submit, trust-set, nft-mint, nft-burn
  server/              fee, info, subscribe
  ledger/              data
```

Each leaf module holds one `Cmd` struct carrying that command's `clap` arguments and a `run` method, so the flags and the code reading them live in the same file. `client`, `output` and `error` hold the plumbing they share.

## Tests

```bash
cargo test -p xrpl-cli                                   # unit tests
cargo test -p xrpl-cli --features integration            # drives the binary against a standalone node on :5005
```
