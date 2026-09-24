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

`new`, `edit`, `sign`, `hash`, `digest`, `blob`, `decode` and `mpt-issuance-id`
make no network calls and need no URL. `autofill` and `submit` are the only two that
reach a node, and neither has a default network — a pipeline that picks one on
your behalf can pick mainnet.

Human-readable output goes to stderr and is silenced by `-q`; stdout carries the
transaction and nothing else, so `jq` always works.

`tx edit` opens the transaction in `$XRPL_EDITOR` (then `$EDITOR`, `$VISUAL`,
`vi`) mid-pipe, for the cases the builder does not model. The editor talks to
`/dev/tty`, not to this process's stdin and stdout — those are carrying the
transaction. With no terminal it exits 5 rather than hanging.

`tx submit` warns on stderr when an MPT `Payment`'s destination holds no
`MPToken` for that issuance, because that fails `tecNO_AUTH` at the ledger and
a 2-of-3 ceremony is an expensive way to discover it. It stays a warning: the
node is the authority on whether a transaction applies.

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

`--seed-file` refuses a file, or a directory, that anyone else can read, and it
is a *pure* stage: it journals nothing and touches no store, so it works on a
read-only container and in CI.

### Or enrol the key once

`key add` and `key generate` record a key under a name you choose and encrypt
the seed under a passphrase. `tx sign --sign-with <name>` then signs without a
seed anywhere in `argv`:

```sh
xrpl key generate issuer --show-secret        # the 16-byte seed IS the backup
xrpl account add issuer --address rISSUER… --network-id 0 \
      --key issuer --default-signer issuer

xrpl tx new Payment --account issuer --field Destination=rDEST… --field Amount=1 \
  | xrpl tx autofill --network testnet \
  | xrpl tx sign --sign-with issuer \
  | xrpl tx submit --wait --network testnet
```

The passphrase comes from a prompt, or from `XRPL_PASSPHRASE` when there is no
terminal — stdin carries the transaction, so it cannot travel that way.

What this claims is **encrypted at rest, plaintext in process memory at signing
time**, and nothing stronger: it defends against a stolen laptop, a record
committed to a repository, a dotfiles sync and a backup, but not against malware
running as you while you sign.

### Which backends this build has

```console
$ xrpl key backends
ephemeral       available      a seed supplied for one invocation, then forgotten (--seed-file, XRPL_SEED, or a prompt)
encrypted-file  available      a passphrase-encrypted blob in the data directory
secure-store    not built in   the same encrypted blob, kept in the OS credential store instead of on the filesystem
watch-only      available      a public key and no secret: it can be recognized, never used
```

`ephemeral` is a backend beside the others, not a special case in the signing
stage. What separates it is that it enrols nothing and journals nothing, which
is what keeps `tx sign --seed-file` a pure crypto stage.

### Where the encrypted blob lives

By default, a file under the data directory. `--secure-store` on `key add` or
`key generate` puts it in the OS credential store instead — Keychain Services,
Credential Manager, or the Secret Service — so it is not on the filesystem at
all, and not in a dotfiles sync or a backup of the home directory.

```bash
cargo install --path xrpl-cli --features secure-store
xrpl key generate issuer --secure-store
```

It is the same encrypted blob either way. **Encrypting first is what makes the
location a swappable detail**: a plaintext secret in a keychain defends against
a stolen disk and a committed dotfile and against nothing else — on Linux any
process on the session bus reads it with no prompt, and on macOS the ACL is
pinned to a code-signing identity, so a `cargo build` binary is never on it and
users get pushed toward "Always Allow".

The feature is off by default because it links a platform backend, and on Linux
a D-Bus client. A binary built without it still *reads* a `secure-store` record
— it lists, `account doctor` reports it, and only using it is refused, naming
the missing backend. Every call has a 30-second deadline, because a Secret
Service that is not running is otherwise a hang.

`key rm` forgets the record and leaves the secret alone. `key rm
--delete-secret` destroys it, wherever it lives; the 16-byte seed is the only
backup there is.

Every signature made through a key record is appended to `signing.log` in the
data directory — the key id, the address and the signing domain. Never the
payload, never the secret.

## Many transactions at once

Three different things get called batching, and only one of them is `Batch`.

**A stream** is the cheap one, and needs no amendment. Every stage already reads
and writes a stream, so the only thing missing was the numbering:

```bash
for d in "${destinations[@]}"; do
  xrpl tx new Payment --account issuer --destination "$d" --amount 25000000
done \
  | xrpl tx autofill --sequence-from-auto --network testnet \
  | xrpl tx sign --sign-with issuer \
  | xrpl tx submit --wait --network testnet
```

`--sequence-from-auto` makes **one** `account_info` call per distinct account and
counts up locally. Without it each line costs its own round trip and they all
come back with the same sequence, so only the first can apply. The counter lives
for the invocation and is never persisted: no sequence is reserved anywhere, and
two invocations racing collide as `tefPAST_SEQ`.

`tx submit` reports every line and exits with the worst result it saw;
`--stop-on-error` abandons the rest at the first failure instead.

## Tickets, the answer available on mainnet today

A ticket detaches a transaction from its account's sequence. That buys two
things a scripted ceremony actually wants: N transactions prepared offline with
no per-transaction round trip, and N slow multisig ceremonies running in
parallel without serialising on the account.

```bash
xrpl tx new TicketCreate --account issuer --ticket-count 3 \
  | xrpl tx autofill --network testnet | xrpl tx sign --sign-with issuer \
  | xrpl tx submit --wait --network testnet

xrpl tx new Payment --account issuer --destination rDEST… --amount 1 \
  | xrpl tx autofill --tickets 165:167 --network testnet | …
```

The results can then be submitted in any order, by different people, at
different times. `--tickets` also sets `Sequence` to 0, because the field is
mandatory and zero is what says "a ticket authorizes this" — rippled answers
`invalidTransaction: Field 'Sequence' is required but missing` otherwise.

## `Batch`, for on-ledger atomicity

XLS-56, and not yet on mainnet.

```bash
xrpl tx new Payment --account issuer --destination rDEST… --amount 1 --field Fee=200 \
  | xrpl tx batch wrap --account issuer --flag tfAllOrNothing --sequence 42 --base-fee 200 \
  | xrpl tx sign --sign-with issuer \
  | xrpl tx submit --wait --network testnet
```

Two to eight inner transactions — rippled answers `Batch has too many inner
transactions` at nine. `batch wrap` is offline: it strips signatures, sets
`Fee: "0"`, `SigningPubKey: ""` and `tfInnerBatchTxn` on each inner, renumbers
the batch account's own inners from the outer's sequence, and refuses a stream
whose lines disagree on `NetworkID` — a batch is the easiest place to fold in a
transaction built for another chain, and the outer signature vouches for every
inner ID it commits to.

`--batch-sign-with` adds `BatchSigners` entries over the `BCH\0` pre-image for
the *other* accounts whose transactions are in the batch. That is a different
array over different bytes from `--sign-with`'s `Signers`, which multisigns the
outer transaction, so both can be present at once.

**The outer transaction returns `tesSUCCESS` even when inner transactions
fail.** So `tx submit` reports each one on stdout and exits 3 if any failed:

```json
{"index":0,"hash":"9959…","result":"tesSUCCESS"}
{"index":1,"hash":"6243…","result":"tecNO_DST_INSUF_XRP"}
```

This is the one place `tx submit`'s stdout is not the transaction it was handed.
Exit 3 on a `Batch` does **not** mean nothing happened, so it is not safe to
resubmit.

## Layout

One module per command, grouped by domain:

```
src/commands/
  global.rs            --url / --network, shared defaults
  tx/                  new, autofill, sign, multisign, merge, submit, hash, digest, blob, decode, …
  account/             add, ls, show, rm, use, doctor  +  info, tx, objects, channels, currencies, lines, nfts
  key/                 add, generate, enrol, export, ls, show, rm
  wallet/              generate, from-seed, faucet, validate
  server/              fee, info, subscribe
  ledger/              data
  rpc.rs               any method, by name
src/store/             account and key records, the encrypted blobs, the journal
src/signer/            resolving a name to something that can sign
```

Each leaf module holds one `Cmd` struct carrying that command's `clap` arguments and a `run` method, so the flags and the code reading them live in the same file. `client`, `output` and `error` hold the plumbing they share.

## Tests

```bash
cargo test -p xrpl-cli                                   # unit tests
cargo test -p xrpl-cli --features integration            # drives the binary against a standalone node on :5005
```

`demo/token-ceremony.sh` is the acceptance gate, and the best worked example of
everything above. It issues an MPT from an account whose master key is disabled
and which is controlled 2-of-3, collecting the quorum both serially (one
operator, one pipeline) and in parallel (three operators, `tx merge`) — using
only `xrpl` commands.

```bash
export XRPL_DATA_DIR=$(mktemp -d) XRPL_CONFIG_DIR=$(mktemp -d)
XRPL_NETWORK=local ./xrpl-cli/demo/token-ceremony.sh all
```

It refuses to run without `XRPL_DATA_DIR`, so it can never write into a real
store. Individual stages (`setup:issuer`, `mint`, `redistribute`, `status`) run
on their own and re-derive what they need.
