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
  local address
  address=$("$XRPL" key show "$name" --json | jq -r .classic_address)

  "$XRPL" account add "$name" \
    --address "$address" \
    --network-id "$EXPECT_NETWORK_ID" \
    --key "$name" \
    --default-signer "$name" >/dev/null

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
  "$XRPL" account add genesis \
    --address "$GENESIS_ADDRESS" \
    --network-id "$EXPECT_NETWORK_ID" \
    --key genesis \
    --default-signer genesis >/dev/null
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
}

setup_governance() {
  say "governance"
  GOVERNANCE=$(make_identity governance)
  note "governance $GOVERNANCE"
  fund "$GOVERNANCE"
  close_ledger

  say "governance: authorize itself to hold the token"
  submit_single governance mptoken-authorize \
    --account "$GOVERNANCE" \
    --mptoken-issuance-id "$MPT" >/dev/null
  close_ledger

  hand_over_to_quorum governance
}

mint() {
  say "mint: 2-of-3 multisigned, collected serially"

  multisign_serial "$STATE/mint.json" payment \
    --account "$ISSUER" \
    --destination "$GOVERNANCE" \
    --amount "100000/$MPT" \
    --memo "mint-period=2026"

  submit_collected "$STATE/mint.json" | jq -r '"   result: " + .meta.TransactionResult' >&2
  close_ledger
}

redistribute() {
  say "redistribute: 2-of-3 multisigned, collected in parallel and merged"

  RECIPIENT=$(make_identity recipient)
  note "recipient $RECIPIENT"
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
    --destination "$RECIPIENT" \
    --amount "25000/$MPT"

  submit_collected "$STATE/redistribute.json" | jq -r '"   result: " + .meta.TransactionResult' >&2
  close_ledger
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

  "$XRPL" key add watcher-only --public-key "$("$XRPL" key show issuer --json | jq -r .public_key)" >/dev/null 2>&1
  "$XRPL" tx new payment --account "$ISSUER" --destination "$GOVERNANCE" --amount 1 \
      --field Fee=12 --field Sequence=1 \
    | "$XRPL" tx sign --sign-with watcher-only >/dev/null 2>&1
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
    status)           setup_signers; setup_issuer; setup_governance; mint; redistribute; status ;;
    all)
      setup_signers
      setup_issuer
      setup_governance
      mint
      redistribute
      status
      check_exit_codes
      check_journal
      say "the ceremony completed"
      ;;
    *) echo "unknown stage '$1'" >&2; exit 1 ;;
  esac
}

main "$@"
