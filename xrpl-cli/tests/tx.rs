//! The `xrpl tx` pipeline.
//!
//! No seed appears in `argv` anywhere in this file: every signing test writes
//! one into a `tempfile` and passes `--seed-file`, so there is nothing for a
//! later migration to clean up and nothing in the process table on CI.

mod common;

use std::io::Write;
use std::path::PathBuf;

use common::{
    ledger_accept, standalone_available, TestEnv, GENESIS_ADDRESS, GENESIS_SEED, STANDALONE_URL,
};
use serde_json::Value;

const DESTINATION: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";

macro_rules! require_standalone {
    ($env:expr) => {
        if !standalone_available($env) {
            eprintln!("skipping: no standalone node at {STANDALONE_URL}");
            return;
        }
    };
}

/// Write the genesis seed into a private file and return its path.
///
/// The directory is 0700 and the file 0600, which is what `--seed-file`
/// insists on — a 0600 file inside a world-readable directory is still one
/// anyone can find.
fn seed_file(env: &TestEnv) -> PathBuf {
    let dir = env.path().join("keys");
    std::fs::create_dir_all(&dir).expect("create key dir");

    let path = dir.join("genesis.seed");
    let mut file = std::fs::File::create(&path).expect("create seed file");
    writeln!(file, "{GENESIS_SEED}").expect("write seed");
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod dir");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    }

    path
}

fn new_payment(env: &TestEnv, drops: &str) -> String {
    let output = env.run(&[
        "tx",
        "new",
        "Payment",
        "--account",
        GENESIS_ADDRESS,
        "--field",
        &format!("Destination={DESTINATION}"),
        "--field",
        &format!("Amount={drops}"),
    ]);
    output.assert_success();
    output.stdout
}

#[test]
fn test_tx_new_is_offline_and_emits_one_line() {
    let env = TestEnv::new();

    // No `--url` anywhere, and nothing listening on port 1 if it tried.
    let payment = new_payment(&env, "10000000");

    assert_eq!(payment.lines().count(), 1, "one transaction is one line");
    let value: Value = serde_json::from_str(payment.trim()).expect("one JSON value");
    assert_eq!(value["TransactionType"], "Payment");
    // Absent and "" are different bytes on the wire.
    assert_eq!(value["SigningPubKey"], "");
    // Drops are string-encoded; a JSON number here is different bytes.
    assert_eq!(value["Amount"], "10000000");
}

#[test]
fn test_the_offline_stages_touch_no_file_and_no_socket() {
    let env = TestEnv::new();
    let seed = seed_file(&env);

    let payment = new_payment(&env, "10000000");
    // Hand-fill what autofill would, so signing can happen with no node at all.
    let prepared = payment.replace(
        r#""SigningPubKey":"""#,
        r#""Fee":"12","Sequence":1,"SigningPubKey":"""#,
    );

    let signed = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap()],
        prepared.as_bytes(),
    );
    signed.assert_success();

    for stage in ["hash", "digest", "blob"] {
        env.run_with_stdin(&["tx", stage], signed.stdout.as_bytes())
            .assert_success();
    }

    // The OFFLINE column in the command surface is a property, not a comment.
    assert!(
        env.is_empty(),
        "an offline stage wrote into the state directory"
    );
}

#[test]
fn test_signing_names_the_address_it_is_signing_as() {
    let env = TestEnv::new();
    let seed = seed_file(&env);

    let payment = new_payment(&env, "10000000");
    let prepared = payment.replace(
        r#""SigningPubKey":"""#,
        r#""Fee":"12","Sequence":1,"SigningPubKey":"""#,
    );

    let signed = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap()],
        prepared.as_bytes(),
    );
    signed.assert_success();

    // A plain `s…` seed always derives secp256k1, so a seed someone believes is
    // Ed25519 signs validly as an address they do not control. Naming the
    // address is the cheapest guard against that.
    signed.assert_stderr_contains(GENESIS_ADDRESS);
    // And it goes to stderr, so it cannot corrupt the pipe.
    assert!(
        !signed
            .stdout
            .contains(GENESIS_ADDRESS.trim_start_matches('r'))
            || {
                let value: Value =
                    serde_json::from_str(signed.stdout.trim()).expect("still valid JSON");
                value["TxnSignature"].is_string()
            }
    );
}

#[test]
fn test_quiet_silences_the_note_but_not_the_artifact() {
    let env = TestEnv::new();
    let seed = seed_file(&env);

    let payment = new_payment(&env, "10000000");
    let prepared = payment.replace(
        r#""SigningPubKey":"""#,
        r#""Fee":"12","Sequence":1,"SigningPubKey":"""#,
    );

    let signed = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap(), "-q"],
        prepared.as_bytes(),
    );
    signed.assert_success();

    assert!(signed.stderr.is_empty(), "stderr: {}", signed.stderr);
    assert!(
        !signed.stdout.is_empty(),
        "-q must not silence the artifact"
    );
}

#[test]
fn test_signing_refuses_a_transaction_with_no_signing_pub_key() {
    let env = TestEnv::new();
    let seed = seed_file(&env);

    let output = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap()],
        br#"{"TransactionType":"Payment","Account":"rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh","Fee":"12","Sequence":1}"#,
    );

    output.assert_code(1);
    output.assert_stderr_contains("SigningPubKey is missing");
}

#[test]
fn test_a_world_readable_seed_file_is_refused() {
    let env = TestEnv::new();
    let path = env.path().join("loose.seed");
    std::fs::write(&path, format!("{GENESIS_SEED}\n")).expect("write");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
    }

    let payment = new_payment(&env, "10000000");
    let output = env.run_with_stdin(
        &["tx", "sign", "--seed-file", path.to_str().unwrap()],
        payment.as_bytes(),
    );

    #[cfg(unix)]
    {
        output.assert_code(1);
        output.assert_stderr_contains("readable by other users");
    }
    #[cfg(not(unix))]
    let _ = output;
}

#[test]
fn test_a_bad_flag_exits_one() {
    let env = TestEnv::new();
    env.run(&["tx", "new", "Payment", "--field", "no-equals-here"])
        .assert_code(1);
}

#[test]
fn test_an_unreachable_node_exits_two() {
    let env = TestEnv::new();
    let payment = new_payment(&env, "10000000");

    // Nothing listens on port 1.
    let output = env.run_with_stdin(
        &["tx", "autofill", "--url", "http://127.0.0.1:1"],
        payment.as_bytes(),
    );

    output.assert_code(2);
}

#[test]
fn test_a_pipeline_stage_refuses_to_pick_a_network() {
    let env = TestEnv::new();
    let payment = new_payment(&env, "10000000");

    // Without this, a bare `tx submit` reaches mainnet with whatever it was
    // handed, because the query commands' NetworkArgs defaults there.
    let output = env.run_with_stdin(&["tx", "autofill"], payment.as_bytes());

    output.assert_code(1);
    output.assert_stderr_contains("no network");
}

#[test]
fn test_hash_refuses_an_unsigned_transaction() {
    let env = TestEnv::new();
    let payment = new_payment(&env, "10000000");

    let output = env.run_with_stdin(&["tx", "hash"], payment.as_bytes());

    output.assert_code(1);
    output.assert_stderr_contains("not knowable");
}

#[test]
fn test_autofill_refuses_an_already_signed_transaction() {
    let env = TestEnv::new();
    let seed = seed_file(&env);

    let payment = new_payment(&env, "10000000");
    let prepared = payment.replace(
        r#""SigningPubKey":"""#,
        r#""Fee":"12","Sequence":1,"SigningPubKey":"""#,
    );
    let signed = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap()],
        prepared.as_bytes(),
    );
    signed.assert_success();

    // Every field autofill sets is a signing field, and XRPL has no fee-bump
    // wrapper, so there is no legal edit after the first signature.
    let output = env.run_with_stdin(
        &["tx", "autofill", "--url", STANDALONE_URL],
        signed.stdout.as_bytes(),
    );

    output.assert_code(1);
    output.assert_stderr_contains("already signed");
}

#[test]
fn test_the_whole_pipeline_validates_against_a_real_ledger() {
    let env = TestEnv::new();
    require_standalone!(&env);
    let _guard = common::blockchain_lock();

    let seed = seed_file(&env);
    let payment = new_payment(&env, "400000000");

    let filled = env.run_with_stdin(
        &["tx", "autofill", "--url", STANDALONE_URL],
        payment.as_bytes(),
    );
    filled.assert_success();

    let signed = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap()],
        filled.stdout.as_bytes(),
    );
    signed.assert_success();

    // The offline hash must match what the node ends up reporting.
    let hashed = env.run_with_stdin(&["tx", "hash"], signed.stdout.as_bytes());
    hashed.assert_success();
    let offline_hash = hashed.stdout_json().as_str().expect("hash").to_string();

    let submitted = env.run_with_stdin(
        &[
            "tx",
            "submit",
            "--wait",
            "--accept-ledger",
            "--url",
            STANDALONE_URL,
        ],
        signed.stdout.as_bytes(),
    );
    submitted.assert_success();

    let result = submitted.stdout_json();
    assert_eq!(result["meta"]["TransactionResult"], "tesSUCCESS");
    assert_eq!(result["validated"], true);
    assert_eq!(
        result["hash"].as_str().expect("node hash"),
        offline_hash,
        "the offline hash must match the one the ledger reports"
    );

    ledger_accept(&env);
}

#[test]
fn test_the_decoded_blob_carries_the_autofilled_fields() {
    let env = TestEnv::new();
    require_standalone!(&env);

    let seed = seed_file(&env);
    let payment = new_payment(&env, "400000000");

    let filled = env.run_with_stdin(
        &["tx", "autofill", "--url", STANDALONE_URL],
        payment.as_bytes(),
    );
    filled.assert_success();

    let signed = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap()],
        filled.stdout.as_bytes(),
    );
    signed.assert_success();

    let blob = env.run_with_stdin(&["tx", "blob"], signed.stdout.as_bytes());
    blob.assert_success();

    let decoded = env.run_with_stdin(&["tx", "decode"], blob.stdout.as_bytes());
    decoded.assert_success();

    // This is the assertion the unsubmittable-blob issue asks for, verbatim.
    let value = decoded.stdout_json();
    for field in ["Fee", "Sequence", "LastLedgerSequence"] {
        assert!(
            !value[field].is_null(),
            "{field} missing from the decoded blob: {value}"
        );
    }

    // The CI container runs [network_id] 0, which is at or below the 1024
    // threshold, so NetworkID must be *absent* rather than present-and-zero.
    assert!(
        value["NetworkID"].is_null(),
        "NetworkID must be omitted on a network at or below 1024: {value}"
    );
}

#[test]
fn test_a_tec_result_exits_three() {
    let env = TestEnv::new();
    require_standalone!(&env);
    let _guard = common::blockchain_lock();

    let seed = seed_file(&env);

    // A fresh destination every run. Reusing a fixed address makes this test
    // depend on whether some earlier run already funded it — which is exactly
    // what happened the first time it was written.
    let fresh = env.run(&["wallet", "generate"]);
    fresh.assert_success();
    let destination = fresh.stdout_json()["classic_address"]
        .as_str()
        .expect("address")
        .to_string();

    // Below the account reserve, so the destination cannot be created and the
    // transaction fails *on the ledger* rather than before it.
    let payment = {
        let output = env.run(&[
            "tx",
            "new",
            "Payment",
            "--account",
            GENESIS_ADDRESS,
            "--field",
            &format!("Destination={destination}"),
            "--field",
            "Amount=1000000",
        ]);
        output.assert_success();
        output.stdout
    };

    let filled = env.run_with_stdin(
        &["tx", "autofill", "--url", STANDALONE_URL],
        payment.as_bytes(),
    );
    filled.assert_success();

    let signed = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap()],
        filled.stdout.as_bytes(),
    );
    signed.assert_success();

    let submitted = env.run_with_stdin(
        &[
            "tx",
            "submit",
            "--wait",
            "--accept-ledger",
            "--url",
            STANDALONE_URL,
        ],
        signed.stdout.as_bytes(),
    );

    // Not 1, and not 0-with-sad-prose. A validated ledger failure is its own
    // thing and a retry wrapper has to be able to see that.
    submitted.assert_code(3);
    let result = submitted.stdout_json();
    assert_eq!(result["meta"]["TransactionResult"], "tecNO_DST_INSUF_XRP");
}

#[test]
fn test_a_stream_of_transactions_stays_a_stream() {
    let env = TestEnv::new();
    let seed = seed_file(&env);

    let one = new_payment(&env, "10000000");
    let prepared = one.replace(
        r#""SigningPubKey":"""#,
        r#""Fee":"12","Sequence":1,"SigningPubKey":"""#,
    );
    let stream = format!("{prepared}{prepared}");

    let signed = env.run_with_stdin(
        &["tx", "sign", "--seed-file", seed.to_str().unwrap()],
        stream.as_bytes(),
    );
    signed.assert_success();

    // One transaction in, one line out, for each. Batching later is iteration,
    // not a redesign.
    assert_eq!(signed.stdout.lines().count(), 2);
    for line in signed.stdout.lines() {
        let value: Value = serde_json::from_str(line).expect("each line is a transaction");
        assert!(value["TxnSignature"].is_string());
    }
}

#[test]
fn test_wallet_generate_emits_json_a_script_can_read() {
    let env = TestEnv::new();

    let output = env.run(&["wallet", "generate"]);
    output.assert_success();

    let value = output.stdout_json();
    assert!(value["classic_address"]
        .as_str()
        .is_some_and(|a| a.starts_with('r')));
    assert!(value["public_key"].is_string());
    // Without --show-secret the seed is not printed at all.
    assert!(value["seed"].is_null());

    let with_secret = env.run(&["wallet", "generate", "--show-secret"]);
    with_secret.assert_success();
    assert!(with_secret.stdout_json()["seed"]
        .as_str()
        .is_some_and(|s| s.starts_with('s')));
}

#[test]
fn test_wallet_generate_save_is_refused_rather_than_silently_doing_nothing() {
    let env = TestEnv::new();

    // It used to print "Saving wallet functionality not implemented yet" and
    // exit 0, which reads as success.
    let output = env.run(&["wallet", "generate", "--save"]);

    output.assert_code(1);
    output.assert_stderr_contains("does not write secrets to disk");
}

#[test]
fn test_mpt_issuance_id_is_derived_offline() {
    let env = TestEnv::new();

    let output = env.run(&[
        "tx",
        "mpt-issuance-id",
        "--account",
        GENESIS_ADDRESS,
        "--sequence",
        "7",
    ]);
    output.assert_success();

    let id = output.stdout_json().as_str().expect("id").to_string();
    assert_eq!(id.len(), 48);
    assert!(id.starts_with("00000007"), "{id}");
}
