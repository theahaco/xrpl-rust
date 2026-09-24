# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [[Incomplete]]

- Performance Benchmarks
- Utility functions

## [[Unreleased]]

### Added

- Support for [XLS-0094D DynamicMPT](https://github.com/XRPLF/XRPL-Standards/pull/583).
- **XLS-0096 Confidential MPT:** support for the [XLS-0096 ConfidentialTransfer amendment](https://github.com/XRPLF/XRPL-Standards/tree/master/XLS-0096-confidential-mpt). Adds the vendored `mpt-crypto` native crypto library via the internal `mpt-crypto` (safe Rust wrappers) and `mpt-crypto-sys` (FFI bindings, statically linked) crates.
- **`GenericRequest`:** an untyped, catch-all request type for XRPL RPC commands that don't have a dedicated typed model yet (e.g. `ledger_accept` on standalone rippled, `server_state`). Accepts a `command` string plus a free-form `params: serde_json::Map<String, Value>` bag; `Serialize` flattens `params` alongside `command`/`id` and strips any collision with those reserved keys. Slots into `XRPLRequest::Generic` and the existing `Request` trait so it flows through `client.request(...)` unchanged.

### Changed

- **Breaking:** the CLI moved out of the library into its own `xrpl-cli` crate (binary still named `xrpl`). `cargo install xrpl-rust --features cli` becomes `cargo install xrpl-cli`; the `cli` feature and its `clap`/`bip39` dependencies are gone from `xrpl-rust`, so library consumers no longer build an argument parser by default (`cli` was in the default feature set). `xrpl::cli` is no longer part of the library API.
- The CLI is now one module per command (`src/commands/<group>/<command>.rs`), each owning its `clap` arguments and a `run` method, with `client`/`output`/`error` modules replacing the free helpers that the old 680-line `execute_command` match shared.
- Commands that reach a node accept `--network mainnet|testnet|devnet|local` alongside `--url`; `--url` wins when both are given and per-command defaults are unchanged (mainnet for queries, testnet for `wallet faucet`, the WebSocket endpoint for `server subscribe`). `-u` is now accepted as the short form of `--url` on every such command rather than only some.
- **Breaking:** `account tx --limit` is now a `u16` (was `u32`), matching the `limit` field of the `account_tx` request.
- **Breaking:** the seven one-shot transaction commands are **removed**: `transaction sign|submit|trust-set|nft-mint|nft-burn` and `account set-flag|clear-flag`. The whole `transaction` group is gone; `xrpl tx` is the only transaction surface. They were deleted rather than shimmed because every blob they emitted was unsubmittable — none of them autofilled `Fee`, `Sequence` or `LastLedgerSequence` — so a shim would have preserved compatibility with output rippled rejects. `xrpl-cli` has never been published, which is what keeps that break free.

  | removed | replacement |
  |---|---|
  | `transaction sign --type T --json '…'` | `xrpl tx new <Type> … \| xrpl tx autofill \| xrpl tx sign` |
  | `transaction submit --tx-blob HEX` | `xrpl tx submit`, with `--wait` to poll to validation |
  | `transaction trust-set -s -i -c -l` | `xrpl tx new TrustSet --limit-amount …` |
  | `transaction nft-mint` | `xrpl tx new NFTokenMint --nftoken-taxon N` |
  | `transaction nft-burn` | `xrpl tx new NFTokenBurn --nftoken-id …` |
  | `account set-flag` / `account clear-flag` | `xrpl tx new AccountSet --set-flag`/`--clear-flag` |

  **Behavior change worth calling out:** `nft-mint` hardcoded `nftoken_taxon(0)` and offered no way to set it. `tx new NFTokenMint` exposes the real field with no default, so a mint that relied on the implicit `0` must now pass `--nftoken-taxon 0` explicitly.

  This also retires the short `-s`. It meant `--seed` on all six of the commands that carried it, and all six are deleted here, so it is retired by deletion rather than deprecation — and must never be reassigned. Repointing it at a key-selection flag would leave every existing script parsing while silently reinterpreting a seed as an account name, which is a wrong-key signature rather than an error.
- `xrpl account tx` gains `history` as a visible alias, and every verb in the group says in its help that it is a ledger query. `xrpl account tx` sitting beside `xrpl tx` never confuses clap, but it does confuse readers.
- **Added:** local **account** and **key** records, under `xrpl account add|ls|show|rm|use|doctor` and `xrpl key add|ls|show|rm`. An account is an address and what is true of it offline; a key is a public key and a pointer to where its secret lives; a signer is resolved at runtime and never persisted. Records live in the **data** directory (`$XRPL_DATA_DIR` → `$XDG_DATA_HOME/xrpl` → `~/.local/share/xrpl`), not the config directory — `~/.config` is what dotfiles tools sync, and committing `alice = rXXXX` permanently links a name to that address's public transaction history. A record never contains key material, and `account ls`/`show` and `key ls`/`show` answer from one file with no network, no credential store and no prompt.
- **Breaking:** every ledger query takes `--account <alias|r-address>` instead of `--address`. `--address` remains as a hidden alias for one minor. `--account` is the only flag in the CLI that resolves an alias — `--destination`, `--issuer`, `--holder` and the address half of `--signer-entry` take literal addresses — and it **refuses anything that parses as a seed**, because at worst a seed there matches a stored alias and signs with the wrong key. Resolution is `--account` → `XRPL_ACCOUNT` → the configured default, with no project-local tier; anything not named explicitly is announced on stderr, and an inherited account is refused on mainnet.
- `xrpl wallet` is hidden and deprecated in favour of `xrpl key` and `xrpl account`. It still works, and will be removed.
- **Added:** an encrypted key store. `xrpl key generate` and `xrpl key add --seed-file|--seed-stdin` enrol a key by encrypting its seed under a passphrase (age, scrypt); `xrpl tx sign --sign-with <key-id>` signs with it. Nothing the CLI writes — record or blob — contains key material in the clear. The honest claim is **encrypted at rest, plaintext in process memory at signing time**, and nothing stronger: `zeroize` disclaims moves, realloc copies, stack spills and swap, so this defends against a stolen laptop, a committed record, a dotfiles sync and a backup, but not against malware running as you while you sign.
- `xrpl key export --i-understand-this-prints-a-secret` prints a seed back. It is gated but it exists: the store is the only copy, this crate has no mnemonic convention, so the 16-byte family seed *is* the backup — a store with no way out strands anyone migrating machines.
- **Fixed:** `encode_for_signing_batch` produced `BatchSigners` signatures no network accepts. It emitted `BCH\0 ++ flags ++ count ++ ids` — an earlier draft of XLS-56, and what xrpl.js's "can create batch blob" test pins — omitting the outer account, the outer sequence and the signer's own account. rippled 3.4.0-rc1 rejects a signature over that with `fails local checks: Invalid signature.`, verified against a live node. The pre-image is now `BCH\0 ++ outer AccountID ++ outer sequence ++ outer Flags ++ inner count ++ each inner ID ++ this signer's AccountID`. **Breaking:** the function takes three more arguments. Keeping the old shape would only have preserved a trap; it had no callers in this tree.
- **Added:** `encode_for_signing_batch_unframed`, the same pre-image without its `BCH\0` prefix. `RawSigner::sign` frames every payload it is given and never accepts pre-framed bytes — that is what stops a signer being talked into signing under the wrong domain — so a caller wanting a batch signature needs the unframed half. A test asserts the two compose back byte-for-byte, because a batch co-signer signing anything else signs something the outer transaction never committed to.
- **Fixed:** a WebSocket subscription died at the node's first keepalive. `AsyncWebSocketClient`'s `Stream` impl mapped `Ping`, `Pong` and raw frames to `UnexpectedMessageType`, ending the stream with an error — and rippled pings its subscribers, so every subscription that outlived the ping interval failed, which is all of them. Control frames are skipped and the socket polled again; tungstenite answers pings itself.
- **Fixed:** one secp256k1 key in sixteen could not be derived or used. `Secp256k1::sign` removed the `00` private-key prefix with `trim_start_matches('0')`, which also removes the key's *own* leading zeros — so a private key whose hex begins with a zero nibble was cut to 63 characters and `SecretKey::from_str` rejected it as `InvalidSecretKey`. Because `derive_keypair` signs a test message to verify itself, this surfaced as **6.25% of secp256k1 key derivations failing outright**, measured at 1,223 failures in 20,000. The prefix is now removed by width, and `SECP256K1_PREFIX` is a `&str` of `"00"` rather than the `char` `'0'` so the mistake is not expressible. Ed25519 was never affected: it already stripped its prefix by width. **Breaking:** `SECP256K1_PREFIX` changes type.
- **Added:** `xrpl tx autofill --sequence-from-auto`, which numbers a stream consecutively from **one** `account_info` call per distinct account. Without it every line cost its own round trip and they all came back with the same sequence, so only the first could apply. The counter is per-invocation and never persisted: there is no reserved sequence anywhere, and racing invocations collide as `tefPAST_SEQ`. The account is each line's own `Account` field, never `--account` or the config default, so a mixed-account stream is numbered correctly per account. The `fee` and `ledger` lookups are now made once per invocation too, rather than once per line.
- **Added:** `xrpl tx autofill --tickets FIRST:LAST`, assigning `TicketSequence` from a range. Tickets are the mainnet-available answer to preparing N transactions with no per-transaction sequence round trip, and to running N slow multisig ceremonies in parallel without serialising on the account's sequence.
- **Fixed:** a ticketed transaction now carries `Sequence: 0` rather than omitting the field. rippled answers `invalidTransaction: Field 'Sequence' is required but missing` to a ticketed transaction with no `Sequence`, so `--no-sequence` — documented as "leave it absent, for a transaction using a ticket" — produced transactions the ledger refused.
- **Fixed:** `xrpl tx autofill` fills `NetworkID`. Its own documentation had always claimed it did; there was no branch for it anywhere in the CLI, and the test asserting the field was absent passed only because the standalone node runs network 0, where absent is correct. On a network with id ≥ 1025 the pipeline emitted transactions with no replay protection at all. The rule is the library's `txn_needs_network_id`, which wants the id **and** a `build_version` of at least 1.11.0, not a second copy of it.
- **Added:** `xrpl tx batch wrap`, folding a stream into one XLS-56 `Batch`. Offline: it strips signatures, sets `Fee: "0"`, `SigningPubKey: ""` and `tfInnerBatchTxn` on each inner, renumbers the batch account's own inners from the outer's sequence, and computes the outer fee as `2 × base + base per outer signature + Σ inner fees` — summed **before** the inner fees are zeroed, because computing it afterwards under-pays by exactly the inner total and fails `telINSUF_FEE_P` after the quorum is spent. `--batch-sign-with` adds `BatchSigners` entries over the `BCH\0` pre-image, sorted by account and excluding the outer signer; it is a different array over different bytes from `--sign-with`'s `Signers`, so both can be present on one Batch.
- **Added:** typed errors for the new failure modes rather than `Error::Other`: `BatchSize` (2 to 8 inner transactions, verified against rippled, which answers `Batch has too many inner transactions` at nine), `BatchInnerSequence` (an inner must carry exactly one of a non-zero `Sequence` or a `TicketSequence`), and `BatchNetworkMismatch` (a batch is the easiest place to fold in a transaction built for another chain).
- **Added:** `xrpl tx submit` reports each inner transaction of a `Batch` as NDJSON on stdout — `{"index":0,"hash":"…","result":"…"}` — and exits 3 when any inner failed. The outer transaction returns `tesSUCCESS` whether or not its inners applied, so reporting only `engine_result` lies; inner transactions land as separate transactions in the same ledger, each with its own result, and are looked up by the ID the outer committed to. This is the one place `tx submit`'s stdout is not the transaction it was handed, and its `--help` says so. Exit 3 on a `Batch` does not mean nothing happened.
- **Added:** `xrpl tx submit --stop-on-error`. By default a stream now submits every line and exits with the worst result it saw; before, any non-`tes` result abandoned the rest of the stream with no way to ask for either behaviour.
- **Fixed:** `xrpl tx sign` writes each signature as it is made rather than buffering the whole stream. A failure on line *k* discarded the signatures of lines 1..*k*-1 from stdout even though the journal had already recorded making them — and for a multisign collection, a signature nobody can see is a signature nobody has.
- **Fixed:** piping into a command that closes early — `xrpl tx new … | head -1` — ended in a Rust panic. `head` closing the pipe is the reader doing its job, not an error.
- **Changed:** `.ci-config/xrpld.cfg` enables `BatchV1_1`, so CI's standalone node can run XLS-56 at all. Without it rippled answers `temDISABLED` to every `Batch`. Note the `feature` RPC is useless for deciding this: on a standalone node it reports `enabled: false` for all 107 amendments, including the 85 that config enables and that demonstrably work.
- **Added:** `--ledger-index` and `--ledger-hash` on `account info|tx|objects|channels|currencies|lines`. Every one of those requests has always supported them and none exposed one, so a query could only read whatever the node considered current — "the balance at ledger 96,000,000" was unaskable. A value that parses as a number is sent as a sequence; anything else is sent as a shortcut, so `validated` and `current` work. `account tx` also gains `--ledger-index-min` and `--ledger-index-max`, whose fields its own test previously asserted were `None` because no flag reached them.
- **Added:** `account info --signer-lists`, the only RPC that answers "who can authorize this account now" — which is what `account doctor` reconciles a local key record against.
- **Changed:** `account objects --type-filter` is now `--type`, named for the field it sets. The old spelling still works as a hidden alias.
- **Fixed:** a node that answered with an error — `lgrNotFound`, `actNotFound` — was printed as though it were data, with exit 0. The transport succeeded, so nothing below the response layer failed and the error body was returned as the result. Query commands now exit 2 and say what the node said. rippled puts the error inside `result` over JSON-RPC and at the top level over WebSocket; both are checked. `xrpl rpc` and the `tx` stages keep showing the node's own answer, error included, because that is what they are for.
- **Changed:** the query commands emit JSON on stdout instead of `{:#?}` under a prose label. Indented by default because a person is usually reading it, compact under `--json` because a script usually is not — both parse, so `jq` works either way and the flag changes only how many newlines there are. The label is now a note on stderr, silenced by `-q`. The set is `account info|tx|objects|channels|currencies|lines|nfts`, `server fee|info`, `ledger data`. The `tx` stages and `xrpl rpc` still have exactly one shape and no flag, deliberately.
- **Changed:** `output::response` takes a value rather than a `Result`. A printer that accepts an error inverts control, and it meant every call site could only report a failure the one way that function chose.
- **Fixed:** `xrpl server subscribe` awaits the socket instead of polling it every 100ms, and emits NDJSON — one compact JSON object per line, the node's own frame passed through. It previously slept in a loop on a synchronous receive, burning a thread for the lifetime of the subscription and adding up to 100ms of latency to every event, and it died on the second message because a stream event is not a response and does not parse as one. Its `--limit` has its own constant rather than borrowing `DEFAULT_PAGINATION_LIMIT`, which sizes pages.
- **Fixed:** `xrpl server fee` awaits the async helper rather than calling the sync one inside a `block_on` to lend it a reactor. `src/ledger`'s wrappers drive their futures with `embassy_futures::block_on`, a bare poll loop with no reactor, so under `std` each carries an undocumented precondition that the caller already be inside a Tokio runtime.
- **Changed:** `xrpl wallet faucet` prints JSON, and the seed only under `--show-secret`. It printed `Wallet`'s `Debug`, which redacts the seed — so a **real funded account** became unrecoverable the moment the process exited, and the redaction protected nothing while costing everything. Without the flag it now says so.
- **Removed:** the `bip39` and `rand` dependencies. Their only use was the `wallet generate --mnemonic` path, which is gone.
- **Added:** `xrpl --help` lists every `XRPL_*` variable and the exit-code table. Most cannot be flags — a passphrase in `argv` is visible to `ps` — so they are documented where they can be found. `--network` gains `env = "XRPL_NETWORK"` and loses to an explicit flag; the `tx` stages deliberately do not read it.
- **Fixed:** the record locator now resolves on Windows. It consulted `$XDG_*` and then `$HOME`, none of which Windows sets, so every command that touched a record failed with "cannot find a home directory" — the CLI was unusable there. Records now go under `%LOCALAPPDATA%\xrpl` and preferences under `%APPDATA%\xrpl`, which is the same split as Unix and for the same reason: `%APPDATA%` roams with a domain profile, and an account record binding a name to a permanent public address is state that should stay on the machine it was made on. `XDG_*` is still honoured first on every platform, for anyone running under MSYS-style tooling who set it deliberately.
- **Fixed:** `xrpl-cli` now selects a `critical-section` implementation (`std`), which is the final binary's job. `xrpl-rust` depends on `embassy-sync` unconditionally — its websocket client uses `CriticalSectionRawMutex` — so the symbols are present whether or not the CLI reaches that code. The GNU and Mach-O linkers drop them as unreachable; MSVC's does not, so the CLI did not link on Windows at all. The cleaner fix is in the library, where `embassy-sync` should sit behind the `embassy-rt` feature rather than being unconditional.
- **Added:** a `secure-store` key backend, behind a Cargo feature of that name. `xrpl key add|generate --secure-store` keeps the encrypted blob in the OS credential store — Keychain Services, Credential Manager, the Secret Service — rather than in a file, so it is not in a dotfiles sync or a backup of the home directory. It is the *same* encrypted blob: encrypting first is what makes the location a swappable detail rather than the security story. The feature is off by default because it links a platform backend and, on Linux, a D-Bus client; a binary built without it still parses and lists such a record and refuses only at the point of use, naming the missing backend, because Cargo features are additive and the shape of a public type must not depend on the feature set. Every credential-store call has a 30-second deadline: a Secret Service that is not running is otherwise a hang, which is the worst answer a CLI can give.
- `xrpl key add` now shows the address a seed derives to and asks before enrolling it, when there is a terminal. A seed derives to exactly one address and nothing warns you if it is not the one you meant — a mistyped path, or the wrong file out of a vault, enrols silently and surfaces much later as a signature from an account nobody expected. `--yes`/`-y` skips it; a script never needs that, because the prompt only appears when a person is there to answer. `key generate` does not ask: the key was made a line ago, so there is nothing to check it against.
- **Added:** `xrpl key backends`, which lists every way this binary can produce a signature and whether it is available here. It exists because the answer is not the same for every build — `secure-store` is behind a Cargo feature and a key record naming it parses either way — so "can this binary use that key" is otherwise a question you answer by trying it. `ephemeral` is listed beside the record-backed ones because it is a backend, not a special case in the signing stage; what separates it is that it enrols nothing and journals nothing.
- **Added:** `xrpl key rm --delete-secret`, which destroys the secret a record points at — file or credential-store entry — rather than only forgetting where it was. Without the flag `key rm` leaves it alone, as before. It is irreversible: the 16-byte family seed is the only backup there is.
- **Added:** `xrpl tx edit`, which opens the transaction in `$XRPL_EDITOR` (then `$EDITOR`, `$VISUAL`, `vi`) mid-pipe. The editor's descriptors are bound to the terminal, not inherited, because stdin carries the transaction and stdout carries the result. Editing a signed transaction invalidates the signature, so it warns before the editor opens rather than after. With no terminal it exits 5.
- `xrpl tx submit` warns when an MPT `Payment`'s destination holds no `MPToken` for that issuance, naming the `MPTokenAuthorize` that fixes it. That failure is `tecNO_AUTH` at the ledger — a burnt fee for a single-signed transaction and a re-collected quorum for a multisigned one. It stays a warning: the node is the authority on whether a transaction applies, so a failed read answers "it holds one" rather than blocking the submission.
- The key-record backends append to a local signing journal (`signing.log` in the data directory): timestamp, key id, address, signing domain. Never the payload, never the secret; a multisign entry records the signer rather than a transaction hash, which is not knowable until the last signature is attached. Writing is best-effort — an unwritable journal warns and never fails a signature. The `ephemeral` backend journals nothing, so `tx sign --seed-file` stays a pure crypto stage that works on a read-only container.
- `XRPL_PASSPHRASE` supplies a passphrase non-interactively. stdin carries the transaction, so a passphrase cannot travel that way; without a terminal, a command exits 6 naming this rather than blocking.
- **Changed:** `xrpl account add --address` is optional when `--key` is given. An account's address derives from its original master public key, so the key record already holds it, and requiring it anyway meant `xrpl key show <id> --json | jq -r .classic_address` at every call site — the ceremony script did exactly that. One distinct address across the named keys is taken and disclosed on stderr; keys that derive to different addresses are refused with both spelled out, because guessing wrong records an account that signs for someone else. `--address` still wins when given, for the regular-key and signer-list-member cases where the two legitimately differ.
- **Added:** part flags on `xrpl key show` (`--address`, `--public-key`, `--algorithm`, `--source`) and `xrpl account show` (`--address`, `--network-id`, `--tag`, `--keys`, `--default-signer`). Each writes that one value bare on stdout and nothing else, so a script needs no JSON parser to read a field out of a record. A field that is not set prints nothing at all and says why on stderr, which keeps `$(xrpl account show alice --tag)` empty rather than the word "none". `--default-signer` is the exception and fails rather than answering emptily: it reports which key the record names — the recorded default, or the only key when there is one — and an account with none, or with several and no default, has no answer to give. That value is what `tx sign --sign-with` takes; `tx sign` does not read account records itself.
- `xrpl account show` names the effective default signer, not only a recorded one. An account with a single key has no `default_signer` field and signs with that key anyway — the commonest record there is — and the block previously said nothing at all about who signs it.
- **Fixed:** `xrpl key ls`, `key show`, `account ls` and `account show` panicked when their reader closed the pipe, the same defect already fixed for the `tx` stages. `xrpl key ls | head -1` ended in a Rust panic message.
- **Fixed:** `SignerError::Unavailable`'s message spliced two sentences together. Every construction site passes a full clause, so `Signer {0} is unavailable on this machine` rendered as `Signer alice is watch-only: it has no keys is unavailable on this machine`, and worse for the credential-store timeout. The prefix is now the label `Signer unavailable:`.

- **Breaking:** every model in `models::requests` and `models::transactions` now constructs through a [`bon`](https://bon-rs.com) builder instead of a positional `new(..)`. `Type::new(a, None, None, ...)` becomes `Type::builder(subject).field(value).build()`; the positional constructors are gone. The subject (the transaction's `account`, a request's primary argument) stays positional on `builder(..)`, every other field is a named setter, `maybe_field(opt)` takes an `Option` you already hold, and string/amount setters accept anything `Into`-convertible (`.fee("12")`). Struct fields, field order, and the serialized wire format are unchanged, so struct-literal construction with `..Default::default()` still works. Adding an optional field to a model is no longer a breaking change for callers.
- **Breaking:** `GenericRequest::new`'s `command` parameter is now `Cow<'a, str>` rather than `impl Into<Cow<'a, str>>`; the builder's `into` conversion replaces it (`GenericRequest::builder("ledger_accept")`).
- `CommonTransactionBuilder::with_fee` and `CommonFields::with_fee` take `impl Into<XRPAmount<'a>>`, so `.with_fee("12")` replaces `.with_fee("12".into())`.
- **Breaking:** `XRPLSubmitAndWaitException::SubmissionFailed` changed from the tuple variant `SubmissionFailed(String)` to the struct variant `SubmissionFailed { result_code: String, message: Option<String> }`. Callers that pattern-matched on `SubmissionFailed(msg)` must now match `SubmissionFailed { result_code, message }` — the code (`temBAD_SIGNATURE`, `tecUNFUNDED_PAYMENT`, `txnNotFound`, ...) is available without substring-parsing the `Display` string. See [#371](https://github.com/XRPLF/xrpl-rust/issues/371) for the follow-up on typing the code itself.
- Both polling-timeout paths in `wait_for_final_transaction_result` now surface `XRPLSubmitAndWaitException::SubmissionTimeout` (retaining the ledger-sequence context). Previously the retry-cap branch (`c > 20`) returned `SubmissionTimeout` while the after-loop fall-through returned `SubmissionFailed { "submission_timeout" }`; `SubmissionFailed` is now reserved for definite rippled result codes.

### Fixed

- `account tx --limit N` set the request's `ledger_index_min` instead of its `limit`, so the flag silently did something unrelated to its name.
- `account clear-flag` built its `AccountSet` with `set_flag`, making it identical to `account set-flag` — it now clears the flag.
- `ledger data` passed `--ledger-index` as the request's `ledger_hash` and `--ledger-hash` as its `ledger_index`; the two were swapped.
- `SubmissionTimeout` `Display` text no longer claims the validated ledger sequence is "greater than" the `LastLedgerSequence` — the retry-cap path can fire while `validated < last`, and the after-loop path also fires on the equality case. Reworded to focus on the outcome (`Transaction not validated before LastLedgerSequence Y (latest validated ledger: X)`) so both paths render correctly.

## [[v1.2.0]]

### Added

- **XLS-33 Multi-Purpose Tokens (MPT):** full support for the [XLS-0033 MPTokensV1 amendment](https://github.com/XRPLF/XRPL-Standards/tree/master/XLS-0033-multi-purpose-tokens).
  - **Binary codec:** `Hash192` type for `MPTokenIssuanceID`; MPT amount encode/decode (UInt64 as base-10 string); `AssetScale` (UInt8) and `MPTAmount`/`MaximumAmount`/`OutstandingAmount` field support.
  - **Amount/Currency:** `MPTAmount` and `MPTCurrency` variants in `Amount`/`Currency` enums; digit-only value validation; `i64::MAX` upper-bound enforcement; `is_mpt()` helper.
  - **Transaction models:** `MPTokenIssuanceCreate`, `MPTokenIssuanceDestroy`, `MPTokenIssuanceSet`, `MPTokenAuthorize`; `Clawback` extended with `MPTAmount` support and optional `Holder` field.
  - **Ledger objects:** `MPToken` and `MPTokenIssuance` with `LockedAmount`, `MPTokenIssuanceMutableFlag` bitmask, and non-null index validation.
  - **Requests:** `AccountObjectType::MptIssuance` and `Mptoken` variants.
  - **Integration tests:** full MPT lifecycle end-to-end (create issuance, holder opt-in, lock/unlock, clawback).
- **XLS-65 Single Asset Vault:** support for the XLS-0065 Single Asset Vault amendment. Adds the `Vault` ledger object; `VaultCreate`, `VaultSet`, `VaultDelete`, `VaultDeposit`, `VaultWithdraw`, and `VaultClawback` transactions; the `vault_info` request and result; `ledger_entry` vault lookup; and the `AccountObjectType::Vault` filter.
- **XLS-47 Price Oracle:** support for the XLS-0047 PriceOracle amendment. Adds the `Oracle` ledger object; `OracleSet` and `OracleDelete` transactions; the `get_aggregate_price` request and result; `ledger_entry` oracle lookup; and the `AccountObjectType::Oracle` filter.
- **XLS-89 MPTokenMetadata:** `utils::mptoken_metadata` helpers to encode, decode, validate, and warn on MPT metadata (`encode_mptoken_metadata`, `decode_mptoken_metadata`, `validate_mptoken_metadata`, `mptoken_metadata_warning`).
- **XLS-39 Clawback:** adds the `Clawback` transaction and the `lsfAllowTrustLineClawback` (`AccountSet`) flag.
- **XLS-70 Credentials:** adds `CredentialCreate`, `CredentialAccept`, and `CredentialDelete` transactions, the `Credential` ledger object, and credential-based `DepositPreauth` fields.
- **Decentralized Identity (DID):** adds the `DIDSet` and `DIDDelete` transactions and the `DID` ledger object.
- New `xrpl::signing` module containing the pure-crypto signing helpers (`sign`, `multisign`, `prepare_transaction`) extracted from `asynch::transaction` and `transaction`. Available with just `core + models + wallet` features (no `helpers`/runtime/client dependency). The legacy paths `asynch::transaction::sign` and `transaction::multisign` are preserved as re-exports for backward compatibility.
- Expanded unit-test coverage and raised CI thresholds: lines `73 → 83`, regions `75 → 85`, functions `67 → 73`.
- Codecov integration with per-PR project (≥83%) and patch (≥80% on new/modified lines) gates.
- Integration-test coverage gate: a CI workflow runs all five integration test binaries under `cargo-llvm-cov`, uploads to codecov under an `integration` flag, and gates the project at ≥65%.

### Changed

- **Breaking:** `Amount::is_issued_currency()` now returns `false` for `MPTAmount`. Previously it returned `!is_xrp()`, so any non-XRP amount yielded `true`. With the introduction of the `MPTAmount` variant, callers that used `is_issued_currency()` as a proxy for "not XRP" must be updated to also check `is_mpt()`. Use the new `is_mpt()` helper for MPT-specific branches.
- Unit-test and integration-test coverage are now scoped via Cargo feature flags rather than path regex. The unit-test workflow builds with `--no-default-features --features std,core,utils,wallet,models`, so integration-territory code (CLI, async clients, sync wrappers, faucet) simply isn't compiled and doesn't appear in the unit coverage report.
- Network-dependent inline tests in `src/asynch/transaction/` and `src/asynch/wallet/` (`test_autofill_txn`, `test_autofill_and_sign`, `test_submit_and_wait`, `test_generate_faucet_wallet`) are now gated behind `feature = "integration"` so `cargo test --release` is hermetic by default.
- Codecov **patch** coverage is now gated per flag (separate `unit` and `integration` sections) rather than a single combined gate.

### Fixed

- Non-cryptographic RNG (`Hc128Rng`) was being used for wallet seed generation; replaced with `OsRng` so all key material is sourced from the OS entropy pool (closes #286).
- `RipplePathFind::destination_amount` changed from `Currency<'a>` to `Amount<'a>` to match the XRPL wire format.
- `NoRippleCheckRole` no longer serializes with the `#[serde(tag = "role")]` discriminator; now emits a plain `snake_case` string matching the XRPL wire format.
- `is_success()` now reports success correctly for responses deserialized into typed `XRPLResult` variants (e.g. `ServerInfo`); it consults the preserved raw result JSON instead of the re-serialized typed value.
- `get_latest_open_ledger_sequence` now uses the `ledger_current` request; it previously sent `ledger { ledger_index: "open" }`, which rippled rejects with `invalidParams`.

## [[v1.1.0]]

- `DepositPreauth` ledger object: `authorize` field changed from `Cow<'a, str>` to `Option<Cow<'a, str>>` to support XLS-70 credential-based preauthorization. The `new()` constructor is unchanged (still accepts non-optional `authorize`), but direct struct construction must wrap the value in `Some(...)`.
- `credential_ids` field on `AccountDelete`, `Payment`, `EscrowFinish`, `PaymentChannelClaim`, and `credentials` on `DepositAuthorized` request changed from `Option<Cow<'a, [Cow<'a, str>]>>` to `Option<Vec<Cow<'a, str>>>` for reliable serde round-trip.

### Added

- Implemented full deserialization from hex binary back to JSON, update `definitions.json` to `xrpl.js` latest, added all codec test fixtures from xrpl.js and implemented tests for all of them.
- Added integration tests for all transaction types, refactored to separate files.
- Added initial XLS-70 Credentials model support (`CredentialCreate`, `CredentialAccept`, `CredentialDelete`, `Credential` ledger object, and credential-based `DepositPreauth` fields).

### Fixed

- Fixed serialization issues for `PathSet`, `Issue`, and `STArray` types.

## [[v1.0.0]]

- Initial production release
- command line interface
- automated market maker
- utility functions
- sidechain support

## [[v.0.6.0]]

- Added CLI interface
- missing network_id member added to server info response
- server_state_duration_us in server info type changed to str

## [[v0.5.0]]

- add missing NFT request models
- add `parse_nftoken_id` and `get_nftoken_id` utility functions
- complete existing result models and add NFT result models
- add transaction `Metadata` models
- fix serialization issue where null values were tried to be serialized
- fix multisigning bug, because `signing_pub_key` is not set for multisigning but it is required, so it's just an empty string
- add transaction response models
- add integration tests with XRPL test net.

## [[v0.4.0]]

- add amm support
  - Transaction models
  - Transaction signing
  - Request models
- add sidechain support
  - Transaction models
  - Transaction signing
- improve errorhandling utilizing thiserror
- simplifying feature flags

## [[v0.3.0]]

- Examples
  - Wallet from seed
  - New wallet generation
  - Client requests
- make `new` methods of models public
- add `AsyncWebSocketClient` and `WebSocketClient`
- add `AsyncJsonRpcClient` and `JsonRpcClient`
- update dependencies
- add devcontainer
- add transaction helpers and signing
- add account helpers
- add ledger helpers
- add wallet helpers

---

## [[v0.2.0-beta]]

### Added

- Request models
- Transaction models
- Ledger models
- Utilize `anyhow` and `thiserror` for models
- Utilities regarding `serde` crate
- Utilities regarding `anyhow` crate

### Changed

- Use `serde_with` to reduce repetitive serialization skip attribute tags
- Use `strum_macros::Display` instead of manual `core::fmt::Display`
- Use `strum_macros::Display` for `CryptoAlgorithm` enum
- Separated `Currency` to `Currency` (`IssuedCurrency`, `XRP`) and `Amount` (`IssuedCurrencyAmount`, `XRPAmount`)
- Make `Wallet` fields public
- Updated crates:
  - secp256k1
  - crypto-bigint
  - serde_with
  - criterion

### Fixed

- Broken documentation link
- Flatten hex exceptions missed from previous pass

---

## [v0.1.1] - 2021-10-28

Initial core release.

### Added

- All Core functionality working with unit tests
