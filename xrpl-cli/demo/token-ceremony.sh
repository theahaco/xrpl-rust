#!/usr/bin/env bash
#
# The acceptance test for the `xrpl tx` pipeline and the account/key records.
#
# Reproduces every on-ledger interaction of theahaco/xrpl-simple-token-test —
# an MPT issued by an account whose master key is disabled and which is
# controlled 2-of-3 — using only `xrpl` commands. If this runs, the project's
# whole flow is scriptable without a browser signer.
#
# Usage:
#   XRPL_NETWORK=local ./xrpl-cli/demo/token-ceremony.sh all
#   XRPL_NETWORK=local ./xrpl-cli/demo/token-ceremony.sh setup:issuer
#
# Only the `local` branch is the enforced CI gate. Testnet has four-second
# ledgers, shared state and a network dependency; it is here because the
# TypeScript project proves that path works, not because CI should rely on it.

set -euo pipefail

# ---------------------------------------------------------------------------
# Environment
# ---------------------------------------------------------------------------

XRPL="${XRPL_BIN:-xrpl}"
NETWORK="${XRPL_NETWORK:-local}"

case "$NETWORK" in
  local)   URL="${XRPL_URL:-http://localhost:5005}"; EXPECT_NETWORK_ID=0 ;;
  testnet) URL="${XRPL_URL:-https://s.altnet.rippletest.net:51234}"; EXPECT_NETWORK_ID=1 ;;
  *) echo "XRPL_NETWORK must be local or testnet, got '$NETWORK'" >&2; exit 1 ;;
esac

# State must never land in a real store. The script refuses to guess: a run
# that silently wrote into someone's ~/.local/share/xrpl and clobbered a live
# alias would be a bad way to find out this file exists.
if [[ -z "${XRPL_DATA_DIR:-}" ]]; then
  echo "XRPL_DATA_DIR is not set." >&2
  echo "This script enrols keys and writes records. Point it at a scratch directory:" >&2
  echo "  export XRPL_DATA_DIR=\$(mktemp -d) XRPL_CONFIG_DIR=\$(mktemp -d)" >&2
  exit 1
fi
export XRPL_CONFIG_DIR="${XRPL_CONFIG_DIR:-$XRPL_DATA_DIR/config}"

# Every signature in this script is non-interactive. There is no controlling
# terminal in CI, and a stage that tried to prompt would exit 6 naming the
# alternative rather than hanging — this is that alternative.
export XRPL_PASSPHRASE="${XRPL_PASSPHRASE:-ceremony passphrase}"

# The genesis account. Standalone has no faucet, so funding is an ordinary
# Payment through the same pipeline as everything else.
GENESIS_SEED="snoPBrXtMeMyMHUVTgbuqAfg1SUTb"
GENESIS_ADDRESS="rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh"

STATE="$XRPL_DATA_DIR/ceremony"
mkdir -p "$STATE"

say()  { printf '\n\033[1m== %s\033[0m\n' "$*" >&2; }
# Stages are not replayable: `setup_issuer` signs with a master key that
# `hand_over_to_quorum` then disables, so running it twice over one state
# directory fails with `tefMASTER_DISABLED` — a confusing way to learn that the
# first run got further than you thought. A marker per stage makes a second run
# resume instead.
done_with() { [[ -f "$STATE/$1.done" ]]; }
mark_done() { : > "$STATE/$1.done"; }
note() { printf '   %s\n' "$*" >&2; }

# ---------------------------------------------------------------------------
# Ledger helpers
# ---------------------------------------------------------------------------

# A standalone node never closes a ledger on its own.
close_ledger() {
  if [[ "$NETWORK" == "local" ]]; then
    "$XRPL" rpc ledger_accept --url "$URL" >/dev/null
  fi
}

# Read the network id from the node and fail loudly if it is not what this
# branch expects. `XRPL_NETWORK` selects a URL; it does not by itself decide
# whether `NetworkID` is emitted, and getting that wrong is a cross-chain
# replay bug rather than a cosmetic one.
assert_network() {
  local actual
  actual=$("$XRPL" rpc server_info --url "$URL" | jq -r '.info.network_id // 0')

  if [[ "$actual" != "$EXPECT_NETWORK_ID" ]]; then
    echo "node reports network_id $actual, expected $EXPECT_NETWORK_ID for '$NETWORK'" >&2
    exit 1
  fi

  note "network_id $actual (NetworkID is $( [[ "$actual" -le 1024 ]] && echo omitted || echo required ))"
}

# Submit and wait for validation.
#
# A standalone node closes a ledger only when asked, so waiting there without
# `--accept-ledger` waits forever. A shared network closes them on its own and
# would refuse the flag. Branching in a function rather than splicing an
# unquoted `$( ... )` into the command line keeps it one readable difference.
submit() {
  if [[ "$NETWORK" == "local" ]]; then
    "$XRPL" tx submit --wait --accept-ledger --url "$URL" "$@"
  else
    "$XRPL" tx submit --wait --url "$URL" "$@"
  fi
}

# Build, autofill, sign with one key, submit, and wait for validation.
submit_single() {
  local key="$1"; shift

  "$XRPL" tx new "$@" \
    | "$XRPL" tx autofill --url "$URL" \
    | "$XRPL" tx sign --sign-with "$key" \
    | submit
}

# ---------------------------------------------------------------------------
# Identities
# ---------------------------------------------------------------------------

# Enrol a key and record the account **before** it receives any XRP.
#
# Enrol-before-fund, always: an account funded before it is recorded can be
# unrecoverable if the process dies in between, and there is no mnemonic here
# to fall back on.
make_identity() {
  local name="$1" algorithm="${2:-secp256k1}"

  if [[ -f "$STATE/$name.address" ]]; then
    cat "$STATE/$name.address"
    return
  fi

  "$XRPL" key generate "$name" --algorithm "$algorithm" >/dev/null

  # Neither --address nor --default-signer: the address is read out of the key
  # record, and a lone key is already the key that signs.
  "$XRPL" account add "$name" \
    --network-id "$EXPECT_NETWORK_ID" \
    --key "$name" >/dev/null

  local address
  address=$("$XRPL" account show "$name" --address)

  echo "$address" > "$STATE/$name.address"
  echo "$address"
}

fund() {
  local address="$1" drops="${2:-500000000}"

  submit_single genesis payment \
    --account "$GENESIS_ADDRESS" \
    --destination "$address" \
    --amount "$drops" >/dev/null
}

enrol_genesis() {
  if "$XRPL" key show genesis >/dev/null 2>&1; then return; fi

  # Materialized by the script, never by the CLI: `xrpl` does not write
  # secrets to disk, and this is the documented way to hand it one.
  ( umask 077 && printf '%s\n' "$GENESIS_SEED" > "$STATE/genesis.seed" )
  chmod 700 "$STATE"

  "$XRPL" key add genesis --seed-file "$STATE/genesis.seed" >/dev/null

  # The one account still told its own address: genesis is a published constant
  # here, not something this script derives from a key it just made.
  "$XRPL" account add genesis \
    --address "$GENESIS_ADDRESS" \
    --network-id "$EXPECT_NETWORK_ID" \
    --key genesis >/dev/null
}

# ---------------------------------------------------------------------------
# Stages
# ---------------------------------------------------------------------------

setup_signers() {
  say "signers"

  # One of the three is Ed25519 on purpose, so the ceremony covers a
  # mixed-algorithm SignerList rather than three identical secp256k1 keys.
  S1=$(make_identity signer-1 secp256k1)
  S2=$(make_identity signer-2 secp256k1)
  S3=$(make_identity signer-3 ed25519)

  note "signer-1 $S1 (secp256k1)"
  note "signer-2 $S2 (secp256k1)"
  note "signer-3 $S3 (ed25519)"
}

# Hand an account over to a 2-of-3 signer list, then disable its master key.
#
# The order matters and is not recoverable if reversed: `asfDisableMaster`
# requires a signer list or a regular key to already be configured, and once
# the master key is off, only the quorum can act.
hand_over_to_quorum() {
  local account="$1"

  say "$account: 2-of-3 signer list"
  submit_single "$account" signer-list-set \
    --account "$(cat "$STATE/$account.address")" \
    --signer-quorum 2 \
    --signer-entry "$S1:1" \
    --signer-entry "$S2:1" \
    --signer-entry "$S3:1" >/dev/null
  close_ledger

  say "$account: disable the master key"
  # Master-key single-sig, and it must be: a multisigned AccountSet carrying
  # SetFlag 4 is refused client-side before any network call.
  submit_single "$account" account-set \
    --account "$(cat "$STATE/$account.address")" \
    --set-flag asfDisableMaster >/dev/null
  close_ledger
}

# Collect a 2-of-3 signature serially, down one pipeline.
#
# One operator holding every key. The signatures accumulate in `Signers`
# because appending provably cannot invalidate an earlier one.
multisign_serial() {
  local out="$1"; shift

  "$XRPL" tx new "$@" \
    | "$XRPL" tx autofill --url "$URL" --signers 2 --no-last-ledger-sequence \
    | "$XRPL" tx sign --multisign --sign-with signer-1 \
    | "$XRPL" tx sign --multisign --sign-with signer-2 \
    > "$out"
}

# Collect a 2-of-3 signature in parallel, then combine.
#
# Three parties who do not share a machine. Each signs the same prepared
# transaction independently; `tx merge` fans them in. Both halves produce
# byte-identical results, which is what makes the ceremony shape a choice
# rather than a constraint.
multisign_parallel() {
  local out="$1"; shift

  "$XRPL" tx new "$@" \
    | "$XRPL" tx autofill --url "$URL" --signers 2 --no-last-ledger-sequence \
    > "$STATE/prepared.json"

  # What a ceremony quotes to its co-signers. Signer-specific, because the
  # signer's own AccountID is part of the bytes.
  note "digest for signer-1: $("$XRPL" tx digest --as-signer "$S1" "$STATE/prepared.json" | tr -d '"' | head -c 40)…"

  "$XRPL" tx sign --multisign --sign-with signer-1 "$STATE/prepared.json" > "$STATE/sig-1.json"
  "$XRPL" tx sign --multisign --sign-with signer-3 "$STATE/prepared.json" > "$STATE/sig-3.json"

  "$XRPL" tx merge "$STATE/sig-1.json" "$STATE/sig-3.json" > "$out"
}

submit_collected() {
  submit "$1"
}

setup_issuer() {
  say "issuer"
  ISSUER=$(make_identity issuer)
  note "issuer $ISSUER"

  if done_with setup_issuer; then
    MPT=$(cat "$STATE/mpt.id")
    note "already set up; issuance $MPT"
    return
  fi

  fund "$ISSUER"
  close_ledger

  say "issuer: create the MPT issuance"
  # tfMPTCanTransfer | tfMPTCanLock | tfMPTCanClawback, which is Flags 98 on
  # the wire. Signed by the master key, which is still live at this point.
  cat > "$STATE/metadata.json" <<'JSON'
{"name":"Ceremony Token","ticker":"CER","desc":"issued by the xrpl-cli acceptance test"}
JSON

  local sequence
  sequence=$("$XRPL" rpc account_info --param account="$ISSUER" --param ledger_index=validated --url "$URL" \
    | jq -r .account_data.Sequence)

  submit_single issuer mptoken-issuance-create \
    --account "$ISSUER" \
    --asset-scale 2 \
    --mptoken-metadata "@$STATE/metadata.json" \
    --flag tfMPTCanTransfer --flag tfMPTCanLock --flag tfMPTCanClawback >/dev/null
  close_ledger

  # The issuance ID is derivable offline — Sequence, big-endian, then the
  # issuer's AccountID — which is how a script learns it without a second
  # lookup, since a submit response carries no metadata.
  MPT=$("$XRPL" tx mpt-issuance-id --account "$ISSUER" --sequence "$sequence" | jq -r .)
  echo "$MPT" > "$STATE/mpt.id"
  note "issuance $MPT"

  hand_over_to_quorum issuer
  mark_done setup_issuer
}

setup_governance() {
  say "governance"
  GOVERNANCE=$(make_identity governance)
  note "governance $GOVERNANCE"

  if done_with setup_governance; then
    note "already set up"
    return
  fi

  fund "$GOVERNANCE"
  close_ledger

  say "governance: authorize itself to hold the token"
  submit_single governance mptoken-authorize \
    --account "$GOVERNANCE" \
    --mptoken-issuance-id "$MPT" >/dev/null
  close_ledger

  hand_over_to_quorum governance
  mark_done setup_governance
}

mint() {
  say "mint: 2-of-3 multisigned, collected serially"

  if done_with mint; then
    note "already minted"
    return
  fi

  multisign_serial "$STATE/mint.json" payment \
    --account "$ISSUER" \
    --destination governance \
    --amount "100000/$MPT" \
    --memo "mint-period=2026"

  submit_collected "$STATE/mint.json" | jq -r '"   result: " + .meta.TransactionResult' >&2
  close_ledger
  mark_done mint
}

redistribute() {
  say "redistribute: 2-of-3 multisigned, collected in parallel and merged"

  RECIPIENT=$(make_identity recipient)
  note "recipient $RECIPIENT"

  if done_with redistribute; then
    note "already redistributed"
    return
  fi

  fund "$RECIPIENT"
  close_ledger

  # The precondition the TypeScript `redistribute` skips. An MPT Payment to a
  # holder who has not authorized the issuance fails `tecNO_AUTH` *after* the
  # whole ceremony has been collected — so authorize first, and never spend a
  # 2-of-3 collection finding out.
  say "recipient: authorize before we spend a ceremony on it"
  submit_single recipient mptoken-authorize \
    --account "$RECIPIENT" \
    --mptoken-issuance-id "$MPT" >/dev/null
  close_ledger

  multisign_parallel "$STATE/redistribute.json" payment \
    --account "$GOVERNANCE" \
    --destination recipient \
    --amount "25000/$MPT"

  submit_collected "$STATE/redistribute.json" | jq -r '"   result: " + .meta.TransactionResult' >&2
  close_ledger
  mark_done redistribute
}

# Whether this node will take a Batch at all.
#
# Not the `feature` RPC: a standalone node reports every amendment disabled,
# including the eighty-five this config switches on, so its answer is noise.
# The only authority is what the node does with a Batch, so hand it one that
# cannot apply — the outer `Sequence` is 1, which genesis consumed on its first
# payment and several stages ago here. A node without XLS-56 stops in preflight
# with `temDISABLED`; a node with it gets as far as `tefPAST_SEQ`. Neither
# applies anything, and neither burns a fee.
#
# Not `submit` either: `--wait` polls for a transaction the node refused before
# it ever reached a ledger, and sixty seconds later reports that it gave up
# rather than what the node actually said.
node_takes_a_batch() {
  local base_fee="$1" probe="$STATE/batch-probe.json" answer

  # `--quiet` throughout: this one is expected to fail, and its commentary
  # interleaved with the real batch's would read as the real batch failing.
  { "$XRPL" tx new payment --quiet --account "$GENESIS_ADDRESS" --destination "$B1" --amount 1
    "$XRPL" tx new payment --quiet --account "$GENESIS_ADDRESS" --destination "$B2" --amount 1
  } | "$XRPL" tx autofill --quiet --url "$URL" --sequence-from-auto --no-last-ledger-sequence \
    | "$XRPL" tx batch wrap --quiet --account genesis --flag tfAllOrNothing \
        --sequence 1 --base-fee "$base_fee" \
    | "$XRPL" tx sign --quiet --sign-with genesis \
    > "$probe"

  # The exit status is not the answer — the probe fails either way. A node that
  # cannot be reached answers nothing at all, which is not `temDISABLED`, so the
  # real batch below is what fails and says so. That is the right way round.
  answer=$("$XRPL" tx submit --url "$URL" "$probe" 2>/dev/null) || true

  [[ "$(jq -r '.engine_result // empty' <<<"$answer")" != "temDISABLED" ]]
}

# Everything the ceremony could not express one transaction at a time: a stream
# numbered from a single lookup, sequence numbers set aside as tickets and spent
# out of order, and two transactions that apply or fail together.
batch() {
  say "streams, tickets and XLS-56"

  B1=$(make_identity batch-1)
  B2=$(make_identity batch-2)
  note "batch-1 $B1"
  note "batch-2 $B2"

  if done_with batch; then
    note "already run"
    return
  fi

  # `tx batch wrap` is offline and so cannot ask, and defaults to the protocol's
  # 10 drops; this node charges 200. The outer `Fee` is a signing field, so a
  # guess that is too low is `telINSUF_FEE_P` after the signature is spent.
  local base_fee
  base_fee=$("$XRPL" rpc fee --url "$URL" | jq -r .drops.base_fee)
  note "reference fee $base_fee drops"

  say "one stream funds two accounts from one account_info"
  # Without `--sequence-from-auto` both lines come back carrying the *same*
  # `Sequence` — one `account_info` each, both answered before either applied —
  # so the second is `tefPAST_SEQ` and only one account is funded.
  { "$XRPL" tx new payment --account "$GENESIS_ADDRESS" --destination "$B1" --amount 500000000
    "$XRPL" tx new payment --account "$GENESIS_ADDRESS" --destination "$B2" --amount 500000000
  } | "$XRPL" tx autofill --url "$URL" --sequence-from-auto \
    | "$XRPL" tx sign --sign-with genesis \
    | submit \
    | jq -r '"   sequence \(.Sequence) funded \(.Destination): \(.meta.TransactionResult)"' >&2
  close_ledger

  say "governance: three tickets, 2-of-3 multisigned"
  # A ticket is a sequence number set aside now to be spent later, by anyone
  # holding the quorum, in any order. Setting them aside is an ordinary
  # transaction, so it needs the quorum like everything else governance has
  # done since its master key went away.
  multisign_serial "$STATE/tickets.json" ticket-create \
    --account "$GOVERNANCE" \
    --ticket-count 3

  submit_collected "$STATE/tickets.json" | jq -r '"   result: " + .meta.TransactionResult' >&2
  close_ledger

  # Read back from the ledger rather than derived from the TicketCreate's own
  # sequence. The node is the authority on which numbers it set aside, and a
  # demo that computed them would be asserting rippled's arithmetic instead of
  # observing it.
  local tickets first second
  tickets=$("$XRPL" rpc account_objects \
      --param account="$GOVERNANCE" --param type=ticket --param ledger_index=validated --url "$URL" \
    | jq -r '[.account_objects[].TicketSequence] | sort | @tsv')
  read -r first second _ <<<"$tickets"
  note "tickets $(tr '\t' ' ' <<<"$tickets")"

  say "two ticketed payments, spent in reverse order"
  # `--tickets` hands them out in stream order, one per line, and sets
  # `Sequence` to 0 on each because a ticket is what authorizes them. Reversing
  # the stream before submitting is the whole point: tickets carry no ordering,
  # so the payment holding the *later* one lands first. Consecutive sequences
  # cannot do that, and the third ticket stays unspent, which they also cannot.
  { "$XRPL" tx new payment --account "$GOVERNANCE" --destination "$RECIPIENT" --amount "100/$MPT"
    "$XRPL" tx new payment --account "$GOVERNANCE" --destination "$RECIPIENT" --amount "200/$MPT"
  } | "$XRPL" tx autofill --url "$URL" --tickets "$first:$second" --signers 2 --no-last-ledger-sequence \
    | "$XRPL" tx sign --multisign --sign-with signer-1 \
    | "$XRPL" tx sign --multisign --sign-with signer-2 \
    > "$STATE/ticketed.json"

  jq -sc 'reverse[]' "$STATE/ticketed.json" \
    | submit \
    | jq -r '"   ticket \(.TicketSequence) spent: \(.meta.TransactionResult)"' >&2
  close_ledger

  say "batch: an authorization and a payment in one atomic step"

  if ! node_takes_a_batch "$base_fee"; then
    note "this node answers temDISABLED to a Batch: it is running without XLS-56"
    note "the tickets above need no amendment, and are the answer mainnet has today"
    mark_done batch
    return
  fi

  local sequence
  sequence=$("$XRPL" rpc account_info --param account="$B1" --param ledger_index=validated --url "$URL" \
    | jq -r .account_data.Sequence)

  # Both inners belong to batch-1, so the outer signature is the only one the
  # batch needs; an inner belonging to another account would want that
  # account's entry in `BatchSigners` as well. `tfAllOrNothing` is the part
  # that matters here — the authorization and the payment both land or neither
  # does, which two separately submitted transactions cannot promise however
  # closely they are submitted together.
  #
  # The outer consumes batch-1's sequence, so `wrap --sequence` renumbers the
  # inners from it; the sequences autofill handed out are the ones that would
  # have collided. And no `LastLedgerSequence` on either: the outer commits to
  # the exact bytes of each inner, so an expiry could not be refreshed
  # afterwards without invalidating the ID the outer vouches for.
  { "$XRPL" tx new mptoken-authorize --account "$B1" --mptoken-issuance-id "$MPT"
    "$XRPL" tx new payment --account "$B1" --destination "$B2" --amount 1000
  } | "$XRPL" tx autofill --url "$URL" --sequence-from-auto --no-last-ledger-sequence \
    | "$XRPL" tx batch wrap --account batch-1 --flag tfAllOrNothing \
        --sequence "$sequence" --base-fee "$base_fee" \
    | "$XRPL" tx sign --sign-with batch-1 \
    | submit \
    | jq -r 'if .index != null then "   inner \(.index): \(.result)" else "   batch: \(.meta.TransactionResult)" end' >&2
  close_ledger

  mark_done batch
}

status() {
  say "status"

  note "outstanding: $("$XRPL" rpc ledger_entry --param mpt_issuance="$MPT" --url "$URL" \
    | jq -r '.node.OutstandingAmount // "0"')"

  # Per-holder balances through `account_objects`: the holder's own MPToken
  # object carries the amount, and filtering by type is the reliable read.
  holds() {
    "$XRPL" rpc account_objects \
        --param account="$1" \
        --param type=mptoken \
        --param ledger_index=validated \
        --url "$URL" \
      | jq -r --arg mpt "$MPT" \
          '[.account_objects[]? | select(.MPTokenIssuanceID == $mpt) | .MPTAmount // "0"] | first // "0"'
  }

  note "governance holds: $(holds "$GOVERNANCE")"
  note "recipient holds:  $(holds "$RECIPIENT")"

  # The signer list, counted from the ledger rather than scraped from prose.
  note "signer list: $("$XRPL" rpc account_objects \
      --param account="$ISSUER" --param type=signer_list --param ledger_index=validated --url "$URL" \
    | jq -r '.account_objects[0] | "\(.SignerEntries | length) entries, quorum \(.SignerQuorum)"')"

  # The local/ledger split, side by side on one address. Only one of these
  # touches the node.
  say "local record beside the ledger"
  "$XRPL" account show issuer >&2
  note "and account doctor reconciles them:"
  "$XRPL" account doctor issuer --ledger --url "$URL" >&2 || true
}

# Deliberately trigger one failure per exit code the script can reach, so the
# acceptance run doubles as the exit-code table's test.
check_exit_codes() {
  say "exit codes"

  local code
  set +e

  "$XRPL" tx new payment --account issuer --destination "$GOVERNANCE" --amount not-an-amount >/dev/null 2>&1
  code=$?; [[ $code -eq 1 ]] || { echo "usage error should exit 1, got $code" >&2; exit 1; }
  note "1 usage"

  "$XRPL" rpc server_info --url http://127.0.0.1:1 >/dev/null 2>&1
  code=$?; [[ $code -eq 2 ]] || { echo "unreachable node should exit 2, got $code" >&2; exit 1; }
  note "2 network"

  "$XRPL" account show no-such-account >/dev/null 2>&1
  code=$?; [[ $code -eq 4 ]] || { echo "missing record should exit 4, got $code" >&2; exit 1; }
  note "4 configuration not found"

  # No controlling terminal here, and `tx edit` needs one. This is the only
  # exit code that is reached by *not* being able to ask a human something.
  { "$XRPL" tx new payment --account "$ISSUER" --destination "$GOVERNANCE" --amount 1 \
      | "$XRPL" tx edit; } >/dev/null 2>&1
  code=$?; [[ $code -eq 5 ]] || { echo "no terminal to prompt on should exit 5, got $code" >&2; exit 1; }
  note "5 declined, or nothing to ask on"

  "$XRPL" key add watcher-only --public-key "$("$XRPL" key show issuer --public-key)" >/dev/null 2>&1
  { "$XRPL" tx new payment --account "$ISSUER" --destination "$GOVERNANCE" --amount 1 \
        --field Fee=12 --field Sequence=1 \
      | "$XRPL" tx sign --sign-with watcher-only; } >/dev/null 2>&1
  code=$?; [[ $code -eq 6 ]] || { echo "unusable signer should exit 6, got $code" >&2; exit 1; }
  note "6 signer unavailable"

  set -e
}

# The journal is the only thing that can answer "was my key used" afterwards,
# and it must never have copied what it signed.
check_journal() {
  say "signing journal"

  local log="$XRPL_DATA_DIR/signing.log"
  [[ -f "$log" ]] || { echo "no signing journal at $log" >&2; exit 1; }

  note "$(wc -l < "$log" | tr -d ' ') signatures recorded"

  if grep -qF "$GENESIS_SEED" "$log"; then
    echo "the journal contains a seed" >&2; exit 1
  fi
  if grep -q "TxnSignature" "$log"; then
    echo "the journal contains a payload" >&2; exit 1
  fi
  note "no seed, no payload"
}

# ---------------------------------------------------------------------------

main() {
  command -v jq >/dev/null || { echo "this script needs jq" >&2; exit 1; }

  assert_network
  enrol_genesis

  case "${1:-all}" in
    setup:issuer)     setup_signers; setup_issuer ;;
    setup:governance) setup_signers; setup_issuer; setup_governance ;;
    mint)             setup_signers; setup_issuer; setup_governance; mint ;;
    redistribute)     setup_signers; setup_issuer; setup_governance; mint; redistribute ;;
    batch)            setup_signers; setup_issuer; setup_governance; mint; redistribute; batch ;;
    status)           setup_signers; setup_issuer; setup_governance; mint; redistribute; batch; status ;;
    all)
      setup_signers
      setup_issuer
      setup_governance
      mint
      redistribute
      batch
      status
      check_exit_codes
      check_journal
      say "the ceremony completed"
      ;;
    *) echo "unknown stage '$1'" >&2; exit 1 ;;
  esac
}

main "$@"
