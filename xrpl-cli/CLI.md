# `xrpl` command reference

Generated from the `clap` definitions. Regenerate with
`cargo run -p xrpl-cli --bin cli-docs > xrpl-cli/CLI.md`; CI fails if this file
and the code disagree.

Hidden commands and flags are left out. `xrpl wallet` and `--seed` are
deprecated and hidden because a seed in `argv` is readable by `ps` and the shell
history; documenting them here would outlive the deprecation.

The fields each `xrpl tx new <type>` accepts are generated separately, from
`definitions.json`, and live in [TX_SURFACE.md](TX_SURFACE.md). This file stops
at the command.

Key material is **encrypted at rest, plaintext in process memory at signing
time**, and nothing stronger: it defends against a stolen laptop, a record
committed to a repository, a dotfiles sync and a backup, but not against malware
running as you while you sign.

## `xrpl`

XRPL command line utility

Subcommands:

- `account` — Account operations
- `key` — Local key records
- `server` — Server operations
- `ledger` — Ledger operations
- `rpc` — Call any rippled RPC directly
- `tx` — Build, sign and submit transactions through a pipeline

Options:

- `-q, --quiet` — Silence human-facing output on stderr. The machine artifact on stdout is never silenced — it is the command's output, not its commentary

```text
Environment:
  XRPL_NETWORK      mainnet | testnet | devnet | local, for the query commands.
                    Loses to --network and --url. `account fund` and the `tx`
                    pipeline stages require an explicit --network or --url.
  XRPL_SEED         a seed for one invocation. Loses to --seed-file.
  XRPL_PASSPHRASE   unlocks an enrolled key without a prompt. The non-interactive
                    path, because stdin carries the transaction.
  XRPL_ACCOUNT      the account to act as, when --account is not given.
  XRPL_DATA_DIR     where account and key records live.
                    Default ~/.local/share/xrpl, %LOCALAPPDATA%\xrpl on Windows.
  XRPL_CONFIG_DIR   where config.toml lives. Default ~/.config/xrpl.
  XRPL_EDITOR       the editor `tx edit` opens. Then EDITOR, VISUAL, vi.

None of these is ever written by this CLI, and none carries key material to
disk. Exit codes: 0 success, 1 usage, 2 network, 3 ledger failure,
4 configuration not found, 5 declined, 6 signer unavailable.
```

## `xrpl account`

Account operations

Subcommands:

- `add` — Record an account (local record, no network)
- `ls` — List account records (local records, no network)
- `show` — Show one account record (local record, no network)
- `rm` — Forget an account record (local record, no network)
- `use` — Choose the default account (local record, no network)
- `doctor` — Report what is wrong with a record (local; --ledger adds a node check)
- `fund` — Fund an existing account from a test-network faucet (no keys required)
- `info` — Get account info from the ledger (ledger query)
- `tx` — Get account transactions (ledger query)
- `objects` — Get account objects: trust lines, offers, signer lists (ledger query)
- `channels` — Get account payment channels (ledger query)
- `currencies` — Get the currencies an account can send or receive (ledger query)
- `lines` — Get account trust lines (ledger query)
- `nfts` — Get account NFTs, XLS-20 (ledger query)

## `xrpl account add`

Record an account (local record, no network)

Arguments:

- `<ALIAS>` — What to call this account

Options:

- `--address <R_ADDRESS>` — The classic r-address. Optional when --key can supply it
- `--network-id <NETWORK_ID>` — Which network it is on. Mainnet is 0
- `--tag <TAG>` — A destination tag carried with the address
- `--key <KEY_ID>` — A key record that can sign for it. Repeatable
- `--default-signer <KEY_ID>` — Which key to use when none is named
- `--force` — Replace an existing record of the same name

## `xrpl account ls`

List account records (local records, no network)

Options:

- `--json` — Emit JSON

## `xrpl account show`

Show one account record (local record, no network)

Arguments:

- `<ALIAS>` — The account alias

Options:

- `--json` — Emit JSON
- `--address` — Print only the classic r-address
- `--network-id` — Print only the network id
- `--tag` — Print only the destination tag
- `--keys` — Print only the key ids, one per line
- `--default-signer` — Print only the key this record names as its default

## `xrpl account rm`

Forget an account record (local record, no network)

Arguments:

- `<ALIAS>` — The account alias

Options:

- `--keys` — Also remove the key records it references

## `xrpl account use`

Choose the default account (local record, no network)

Arguments:

- `[ALIAS]` — The account alias. Omit with --clear to unset the default

Options:

- `--clear` — Unset the default

## `xrpl account doctor`

Report what is wrong with a record (local; --ledger adds a node check)

Arguments:

- `[ALIAS]` — The account to check. Omit to check every record

Options:

- `--ledger` — Also reconcile against the ledger. Requires a node
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Emit JSON

## `xrpl account fund`

Fund an existing account from a test-network faucet (no keys required)

Arguments:

- `<ALIAS_OR_ADDRESS>` — The existing account alias or classic r-address to fund

Options:

- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local)
- `--faucet-url <URL>` — Override the faucet's HTTP endpoint (the full funding URL)
- `--timeout <TIMEOUT>` — Maximum seconds for the request and validated balance increase (1–3600) (default: 60)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl account info`

Get account info from the ledger (ledger query)

Options:

- `--account <ALIAS_OR_ADDRESS>` — The account: a recorded alias, or a literal r-address
- `--signer-lists` — Include the account's signer list, if it has one
- `--ledger-index <INDEX_OR_SHORTCUT>` — The ledger to read: a sequence number, or validated|closed|current
- `--ledger-hash <HASH>` — The ledger to read, by its hash
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl account tx`

Get account transactions (ledger query)

Aliased as `history`: `xrpl account tx` sits next to `xrpl tx`, and while clap never has to disambiguate them, a reader does.

Options:

- `--account <ALIAS_OR_ADDRESS>` — The account: a recorded alias, or a literal r-address
- `--limit <LIMIT>` — Limit the number of transactions returned (default: 10)
- `--ledger-index-min <INDEX>` — Only transactions at or after this ledger index
- `--ledger-index-max <INDEX>` — Only transactions at or before this ledger index
- `--ledger-index <INDEX_OR_SHORTCUT>` — The ledger to read: a sequence number, or validated|closed|current
- `--ledger-hash <HASH>` — The ledger to read, by its hash
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl account objects`

Get account objects: trust lines, offers, signer lists (ledger query)

Options:

- `--account <ALIAS_OR_ADDRESS>` — The account: a recorded alias, or a literal r-address
- `--type <TYPE>` — Type of objects to return (all, offer, state, etc.)
- `--limit <LIMIT>` — Limit the number of objects returned (default: 10)
- `--ledger-index <INDEX_OR_SHORTCUT>` — The ledger to read: a sequence number, or validated|closed|current
- `--ledger-hash <HASH>` — The ledger to read, by its hash
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl account channels`

Get account payment channels (ledger query)

Options:

- `--account <ALIAS_OR_ADDRESS>` — The account: a recorded alias, or a literal r-address
- `--destination-account <DESTINATION_ACCOUNT>` — Destination account to filter channels
- `--limit <LIMIT>` — Limit the number of channels returned (default: 10)
- `--ledger-index <INDEX_OR_SHORTCUT>` — The ledger to read: a sequence number, or validated|closed|current
- `--ledger-hash <HASH>` — The ledger to read, by its hash
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl account currencies`

Get the currencies an account can send or receive (ledger query)

Options:

- `--account <ALIAS_OR_ADDRESS>` — The account: a recorded alias, or a literal r-address
- `--ledger-index <INDEX_OR_SHORTCUT>` — The ledger to read: a sequence number, or validated|closed|current
- `--ledger-hash <HASH>` — The ledger to read, by its hash
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl account lines`

Get account trust lines (ledger query)

Options:

- `--account <ALIAS_OR_ADDRESS>` — The account: a recorded alias, or a literal r-address
- `--peer <PEER>` — Peer account to filter trust lines
- `--limit <LIMIT>` — Limit the number of trust lines returned (default: 10)
- `--ledger-index <INDEX_OR_SHORTCUT>` — The ledger to read: a sequence number, or validated|closed|current
- `--ledger-hash <HASH>` — The ledger to read, by its hash
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl account nfts`

Get account NFTs, XLS-20 (ledger query)

Options:

- `--account <ALIAS_OR_ADDRESS>` — The account: a recorded alias, or a literal r-address
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl key`

Local key records

Subcommands:

- `add` — Record a key (local record, no network)
- `generate` — Generate a key and enrol it (local record, no network)
- `export` — Print a key's seed (local record, no network)
- `ls` — List key records (local records, no network)
- `show` — Show one key record (local record, no network)
- `rm` — Remove a key record (local record, no network)
- `backends` — List the signing backends this binary was built with (no network)

## `xrpl key add`

Record a key (local record, no network)

Arguments:

- `<ID>` — What to call this key. You choose the name; `--key` uses it

Options:

- `--public-key <HEX>` — Record a public key only: it can be recognized, never used
- `--seed-file <PATH>` — Enrol the seed in this file. It is encrypted before being stored
- `--seed-stdin` — Enrol a seed read from stdin
- `--force` — Replace an existing record of the same name
- `--secure-store` — Keep the encrypted blob in the OS credential store, not in a file
- `-y, --yes` — Enrol without confirming the derived address

## `xrpl key generate`

Generate a key and enrol it (local record, no network)

Arguments:

- `<ID>` — What to call the key

Options:

- `--show-secret` — Print the seed on stdout
- `--algorithm <ALGORITHM>` — Which curve to use (values: secp256k1, ed25519)
- `--force` — Replace an existing record of the same name
- `--secure-store` — Keep the encrypted blob in the OS credential store, not in a file

## `xrpl key export`

Print a key's seed (local record, no network)

Arguments:

- `<ID>` — The key id

Options:

- `--i-understand-this-prints-a-secret` — Required. Says out loud what this does

## `xrpl key ls`

List key records (local records, no network)

Options:

- `--json` — Emit JSON

## `xrpl key show`

Show one key record (local record, no network)

Arguments:

- `<ID>` — The key id

Options:

- `--json` — Emit JSON
- `--address` — Print only the address this key derives to
- `--public-key` — Print only the public key
- `--algorithm` — Print only the curve: secp256k1 or ed25519
- `--source` — Print only where the secret lives

## `xrpl key rm`

Remove a key record (local record, no network)

Arguments:

- `<ID>` — The key id

Options:

- `--force` — Remove it even though accounts still reference it
- `--delete-secret` — Also destroy the encrypted secret this record points at

## `xrpl key backends`

List the signing backends this binary was built with (no network)

Options:

- `--json` — Emit JSON on stdout

## `xrpl server`

Server operations

Subcommands:

- `fee` — Get current network fee
- `info` — Get server info
- `subscribe` — Subscribe to ledger events

## `xrpl server fee`

Get current network fee

Options:

- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl server info`

Get server info

Options:

- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl server subscribe`

Subscribe to ledger events

Options:

- `--stream <STREAM>` — Stream type to subscribe to (ledger, transactions, validations) (default: ledger)
- `--limit <LIMIT>` — Number of events to receive before exiting (0 for unlimited) (default: 10)
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)

## `xrpl ledger`

Ledger operations

Subcommands:

- `data` — Get ledger data

## `xrpl ledger data`

Get ledger data

Options:

- `--ledger-index <LEDGER_INDEX>` — Ledger index (empty for latest)
- `--ledger-hash <LEDGER_HASH>` — Ledger hash (empty for latest)
- `--limit <LIMIT>` — Limit the number of objects returned (default: 10)
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)
- `--json` — Print one line of compact JSON instead of an indented document

## `xrpl rpc`

Call any rippled RPC directly

Arguments:

- `<COMMAND>` — The rippled RPC command name, e.g. `server_info` or `ledger_accept`

Options:

- `--param <KEY=VALUE>` — A request parameter, as `key=value`. Repeatable
- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local) (env: XRPL_NETWORK)

## `xrpl tx`

Build, sign and submit transactions through a pipeline

Subcommands:

- `new` — Build an unsigned transaction (OFFLINE)
- `autofill` — Fill in Fee, Sequence, LastLedgerSequence and NetworkID
- `batch` — Fold a stream into one XLS-56 Batch (OFFLINE)
- `edit` — Open a transaction in $EDITOR (OFFLINE)
- `sign` — Sign a transaction (OFFLINE)
- `merge` — Combine independently signed copies of one transaction (OFFLINE)
- `submit` — Encode and submit a transaction
- `hash` — Print the transaction ID (OFFLINE, signed only)
- `digest` — Print what a signer actually signs (OFFLINE)
- `blob` — Render a transaction as a hex blob (OFFLINE)
- `decode` — Parse a hex blob back into JSON (OFFLINE)
- `mpt-issuance-id` — Derive an MPT issuance ID (OFFLINE)
- `fields` — List the fields and flags a transaction type accepts (OFFLINE)

## `xrpl tx new`

Build an unsigned transaction (OFFLINE)

One subcommand per transaction type, each with the fields that type
accepts: see [TX_SURFACE.md](TX_SURFACE.md).

## `xrpl tx autofill`

Fill in Fee, Sequence, LastLedgerSequence and NetworkID

Arguments:

- `[TX]` — Transaction JSON, a file containing it, or `-`/empty for stdin

Options:

- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local)
- `--fee <FEE>` — Use this fee instead of asking the node
- `--sequence <SEQUENCE>` — Use this sequence instead of asking the node
- `--no-sequence` — Set `Sequence` to 0, for a transaction using a ticket
- `--sequence-from-auto` — Number a stream consecutively from one `account_info` call per account
- `--tickets <FIRST:LAST>` — Assign `TicketSequence` from a range, one per line: `--tickets 165:167`
- `--signers <SIGNERS>` — Scale the fee for a multisigned transaction with this many signers
- `--no-last-ledger-sequence` — Leave `LastLedgerSequence` absent

## `xrpl tx batch`

Fold a stream into one XLS-56 Batch (OFFLINE)

Subcommands:

- `wrap` — Fold a stream into one Batch transaction (OFFLINE)

## `xrpl tx batch wrap`

Fold a stream into one Batch transaction (OFFLINE)

Arguments:

- `[TX]` — Transaction JSON, a file containing it, or `-`/empty for stdin

Options:

- `--account <ALIAS_OR_ADDRESS>` — The account that submits the batch
- `--flag <NAME>` — The batch mode: tfAllOrNothing, tfOnlyOne, tfUntilFailure, tfIndependent
- `--sequence <SEQUENCE>` — `Sequence` for the outer transaction
- `--signers <SIGNERS>` — Scale the outer fee for a multisigned outer transaction
- `--base-fee <BASE_FEE>` — The network's reference fee, in drops (default: 10)
- `--batch-sign-with <KEY_ID>` — Sign the batch pre-image with this recorded key. Repeatable
- `--check-amendment <URL>` — Ask a node whether XLS-56 is usable before spending a signature

## `xrpl tx edit`

Open a transaction in $EDITOR (OFFLINE)

Arguments:

- `[TX]` — Transaction JSON, a file containing it, or `-`/empty for stdin

## `xrpl tx sign`

Sign a transaction (OFFLINE)

Arguments:

- `[TX]` — Transaction JSON, a file containing it, or `-`/empty for stdin

Options:

- `--seed-file <PATH>` — Path to a file whose first line is a seed
- `-k, --key <KEY_ID>` — Sign with a recorded key, by id
- `--multisign` — Add a signature to `Signers` instead of signing the transaction outright
- `-y, --yes` — Sign a stream without confirming what is in it

## `xrpl tx merge`

Combine independently signed copies of one transaction (OFFLINE)

Arguments:

- `<TX>` — The signed copies to combine. Each is transaction JSON, a file containing it, or `-` for stdin (repeatable)

## `xrpl tx submit`

Encode and submit a transaction

Arguments:

- `[TX]` — Transaction JSON, a file containing it, or `-`/empty for stdin

Options:

- `-u, --url <URL>` — The XRPL node URL. Takes precedence over --network
- `--network <NETWORK>` — A named network to use instead of spelling out --url (values: mainnet, testnet, devnet, local)
- `--wait` — Poll until the transaction is in a validated ledger
- `--accept-ledger` — Close a ledger between polls, for a standalone node
- `--stop-on-error` — Abandon the rest of the stream at the first failure

## `xrpl tx hash`

Print the transaction ID (OFFLINE, signed only)

Arguments:

- `[TX]` — Transaction JSON, a file containing it, or `-`/empty for stdin

## `xrpl tx digest`

Print what a signer actually signs (OFFLINE)

Arguments:

- `[TX]` — Transaction JSON, a file containing it, or `-`/empty for stdin

Options:

- `--as-signer <R_ADDRESS>` — Render the multisign pre-image for this signer's address

## `xrpl tx blob`

Render a transaction as a hex blob (OFFLINE)

Arguments:

- `[TX]` — Transaction JSON, a file containing it, or `-`/empty for stdin

## `xrpl tx decode`

Parse a hex blob back into JSON (OFFLINE)

Arguments:

- `[BLOB]` — A hex transaction blob, a file containing one, or `-`/empty for stdin

## `xrpl tx mpt-issuance-id`

Derive an MPT issuance ID (OFFLINE)

Options:

- `--account <R_ADDRESS>` — The issuing account
- `--sequence <SEQUENCE>` — The `Sequence` of the `MPTokenIssuanceCreate` that created it

## `xrpl tx fields`

List the fields and flags a transaction type accepts (OFFLINE)

Arguments:

- `[TYPE]` — The transaction type. Omit to list every type

Options:

- `--json` — Emit the table as JSON
