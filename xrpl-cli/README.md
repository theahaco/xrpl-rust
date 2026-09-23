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

## The `tx` pipeline

Every stage reads one machine artifact on stdin and writes one on stdout, so
they compose:

```sh
xrpl tx new Payment \
      --account rISSUER… --field Destination=rDEST… --field Amount=10000000 \
  | xrpl tx autofill --network testnet \
  | xrpl tx sign --seed-file ~/keys/alice \
  | xrpl tx submit --wait --network testnet
```

`new`, `sign`, `hash`, `digest`, `blob`, `decode` and `mpt-issuance-id` make no
network calls and need no URL. `autofill` and `submit` are the only two that
reach a node, and neither has a default network — a pipeline that picks one on
your behalf can pick mainnet.

Human-readable output goes to stderr and is silenced by `-q`; stdout carries the
transaction and nothing else, so `jq` always works.

### Supplying a seed

`--seed-file` reads a file you own; the CLI never writes one. It composes with
any secret manager through process substitution:

```sh
xrpl tx sign --seed-file <(op read op://vault/issuer/seed)
XRPL_SEED=$(op read op://vault/issuer/seed) xrpl tx sign
```

For a standalone node, materialize the well-known genesis seed yourself — there
is deliberately no CLI command that writes a secret to disk:

```sh
umask 077 && printf '%s\n' "$GENESIS_SEED" > genesis.seed   # never via the CLI
xrpl tx sign --seed-file genesis.seed
```

Encrypted-at-rest key records are a later change. What ships today is
**plaintext on disk, protected by the permissions you give it** — `--seed-file`
refuses a file, or a directory, that anyone else can read.

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
