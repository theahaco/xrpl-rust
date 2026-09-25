//! Drive signer selection through the real CLI, with isolated offline stores.

mod common;

use common::{CliOutput, TestEnv, GENESIS_ADDRESS, GENESIS_SEED};
use serde_json::{json, Value};
use xrpl::core::binarycodec::{encode_for_multisigning, encode_for_signing};
use xrpl::core::keypairs::is_valid_message;
use xrpl::wallet::Wallet;

const DESTINATION: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";

fn transaction(address: &str) -> Value {
    json!({
        "TransactionType": "Payment", "Account": address,
        "Destination": DESTINATION, "Amount": "1", "Fee": "12",
        "Sequence": 1, "SigningPubKey": ""
    })
}

fn fixture() -> TestEnv {
    let env = TestEnv::new().env("XRPL_PASSPHRASE", "signer selection test");
    env.run_with_stdin(
        &["key", "add", "alice-master", "--seed-stdin", "--yes"],
        GENESIS_SEED.as_bytes(),
    )
    .assert_success();
    env.run(&["account", "add", "alice", "--key", "alice-master"])
        .assert_success();
    env
}

fn sign(env: &TestEnv, args: &[&str], tx: &Value) -> CliOutput {
    let mut command = vec!["tx", "sign"];
    command.extend_from_slice(args);
    env.run_with_stdin(&command, tx.to_string().as_bytes())
}

fn verify(tx: &Value) {
    let bytes = hex::decode(encode_for_signing(tx).expect("signing bytes")).unwrap();
    assert!(is_valid_message(
        &bytes,
        tx["TxnSignature"].as_str().unwrap(),
        tx["SigningPubKey"].as_str().unwrap()
    ));
}

fn watch_key(env: &TestEnv) {
    let wallet = Wallet::new(GENESIS_SEED, 0).unwrap();
    env.run(&["key", "add", "watcher", "--public-key", &wallet.public_key])
        .assert_success();
}

fn assert_unsigned_failure(env: &TestEnv, output: CliOutput, code: i32, message: &str) {
    output.assert_code(code);
    output.assert_stderr_contains(message);
    assert!(output.stdout.is_empty(), "{}", output.stdout);
    assert!(!env.data_dir().join("signing.log").exists());
}

#[test]
fn sole_key_long_short_and_legacy_selectors_produce_the_same_valid_signature() {
    let env = fixture();
    // Exercise the real builder: Account is an address, not the account alias
    // or the (deliberately different) key id.
    let built = env.run(&[
        "tx",
        "new",
        "payment",
        "--account",
        "alice",
        "--destination",
        DESTINATION,
        "--amount",
        "1",
        "--field",
        "Fee=12",
        "--field",
        "Sequence=1",
    ]);
    built.assert_success();
    let tx = built.stdout_json();
    assert_eq!(tx["Account"], GENESIS_ADDRESS);
    let automatic = sign(&env, &[], &tx);
    automatic.assert_success();
    automatic.assert_stderr_contains(GENESIS_ADDRESS);
    let expected = automatic.stdout_json();
    verify(&expected);
    for flag in ["--key", "-k", "--sign-with"] {
        let explicit = sign(&env, &[flag, "alice-master"], &tx);
        explicit.assert_success();
        assert_eq!(explicit.stdout_json(), expected);
    }
    let journal = std::fs::read_to_string(env.data_dir().join("signing.log")).unwrap();
    assert_eq!(journal.lines().count(), 4);
    assert!(!journal.contains(GENESIS_SEED));
}

#[test]
fn help_advertises_one_key_and_repeated_selectors_are_rejected() {
    let env = TestEnv::new();
    let help = env.run(&["tx", "sign", "--help"]);
    help.assert_success();
    assert!(
        help.stdout.contains("-k, --key <KEY_ID>"),
        "{}",
        help.stdout
    );
    assert!(!help.stdout.contains("--sign-with"));
    assert!(!help.stdout.contains("Repeatable"));
    for second in ["--key", "-k", "--sign-with"] {
        let output = sign(
            &env,
            &["--key", "alice", second, "bob"],
            &transaction(GENESIS_ADDRESS),
        );
        assert!(!output.success());
        assert!(output.stdout.is_empty());
        output.assert_stderr_contains("cannot be used multiple times");
    }
    assert!(env.is_empty());
}

#[test]
fn several_keys_require_a_choice_or_a_recorded_default() {
    let env = fixture();
    watch_key(&env);
    let args = [
        "account",
        "add",
        "alice",
        "--force",
        "--key",
        "watcher",
        "--key",
        "alice-master",
    ];
    env.run(&args).assert_success();
    assert_unsigned_failure(
        &env,
        sign(&env, &[], &transaction(GENESIS_ADDRESS)),
        1,
        "--key",
    );
    let mut with_default = args.to_vec();
    with_default.extend(["--default-signer", "alice-master"]);
    env.run(&with_default).assert_success();
    let signed = sign(&env, &[], &transaction(GENESIS_ADDRESS));
    signed.assert_success();
    verify(&signed.stdout_json());
    let journal = std::fs::read_to_string(env.data_dir().join("signing.log")).unwrap();
    assert!(journal.contains("alice-master"));
    assert!(!journal.contains("watcher"));
}

#[test]
fn unavailable_default_does_not_fall_back_to_another_key() {
    let env = fixture();
    watch_key(&env);
    env.run(&[
        "account",
        "add",
        "alice",
        "--force",
        "--key",
        "alice-master",
        "--key",
        "watcher",
        "--default-signer",
        "watcher",
    ])
    .assert_success();
    assert_unsigned_failure(
        &env,
        sign(&env, &[], &transaction(GENESIS_ADDRESS)),
        6,
        "watch-only",
    );
    sign(&env, &["-k", "alice-master"], &transaction(GENESIS_ADDRESS)).assert_success();
}

#[test]
fn explicit_sources_bypass_account_lookup_and_key_takes_precedence() {
    let env = fixture();
    std::fs::write(env.data_dir().join("accounts/alice.toml"), "not valid toml").unwrap();
    let with_bad_seed = TestEnv::reusing(env.path())
        .env("XRPL_PASSPHRASE", "signer selection test")
        .env("XRPL_SEED", "not a seed");
    let explicit = sign(
        &with_bad_seed,
        &["-k", "alice-master"],
        &transaction(GENESIS_ADDRESS),
    );
    explicit.assert_success();
    verify(&explicit.stdout_json());
    let before = std::fs::read(env.data_dir().join("signing.log")).unwrap();
    let ephemeral = TestEnv::reusing(env.path()).env("XRPL_SEED", GENESIS_SEED);
    let output = sign(&ephemeral, &[], &transaction(GENESIS_ADDRESS));
    output.assert_success();
    verify(&output.stdout_json());
    assert_eq!(
        before,
        std::fs::read(env.data_dir().join("signing.log")).unwrap()
    );
}

#[test]
fn cli_and_environment_default_accounts_do_not_select_the_signer() {
    let env = fixture();
    env.run(&["account", "add", "other", "--address", DESTINATION])
        .assert_success();
    env.run(&["account", "use", "other"]).assert_success();
    let env = env.env("XRPL_ACCOUNT", "other");
    let output = sign(&env, &[], &transaction(GENESIS_ADDRESS));
    output.assert_success();
    verify(&output.stdout_json());
}

#[test]
fn regular_keys_require_explicit_selection_even_if_the_cached_address_is_wrong() {
    let env = fixture();
    env.run(&[
        "account",
        "add",
        "alice",
        "--force",
        "--address",
        DESTINATION,
        "--key",
        "alice-master",
    ])
    .assert_success();
    let tx = transaction(DESTINATION);
    assert_unsigned_failure(
        &env,
        sign(&env, &[], &tx),
        1,
        "regular keys require explicit --key",
    );
    // A stale or hand-edited address cache must not permit implicit regular-key signing.
    let path = env.data_dir().join("keys/alice-master.toml");
    let record = std::fs::read_to_string(&path)
        .unwrap()
        .replace(GENESIS_ADDRESS, DESTINATION);
    std::fs::write(path, record).unwrap();
    assert_unsigned_failure(
        &env,
        sign(&env, &[], &tx),
        1,
        "regular keys require explicit --key",
    );
    let explicit = sign(&env, &["-k", "alice-master"], &tx);
    explicit.assert_success();
    verify(&explicit.stdout_json());
}

#[test]
fn duplicate_address_records_are_ambiguous() {
    let env = fixture();
    env.run(&["account", "add", "another-alias", "--key", "alice-master"])
        .assert_success();
    assert_unsigned_failure(
        &env,
        sign(&env, &[], &transaction(GENESIS_ADDRESS)),
        1,
        "several saved accounts match",
    );
    sign(&env, &["-k", "alice-master"], &transaction(GENESIS_ADDRESS)).assert_success();
}

#[test]
fn watch_only_and_missing_records_do_not_prompt_for_an_alternate_seed() {
    let env = fixture();
    let tx = transaction(GENESIS_ADDRESS);
    let path = env.data_dir().join("accounts/alice.toml");
    let original = std::fs::read_to_string(&path).unwrap();
    // A default pointing outside the account's keys is invalid even if the key exists.
    std::fs::write(
        &path,
        format!("{original}\ndefault_signer = \"unassociated\"\n"),
    )
    .unwrap();
    assert_unsigned_failure(&env, sign(&env, &[], &tx), 4, "key on this account");
    std::fs::write(&path, &original).unwrap();
    std::fs::remove_file(env.data_dir().join("keys/alice-master.toml")).unwrap();
    assert_unsigned_failure(&env, sign(&env, &[], &tx), 4, "No such key");
    env.run(&[
        "account",
        "add",
        "alice",
        "--force",
        "--address",
        GENESIS_ADDRESS,
    ])
    .assert_success();
    assert_unsigned_failure(&env, sign(&env, &[], &tx), 6, "watch-only");
}

#[test]
fn unrecorded_account_retains_the_seed_prompt_path_without_creating_a_store() {
    let env = TestEnv::new();
    let output = sign(&env, &[], &transaction(GENESIS_ADDRESS));
    assert_unsigned_failure(&env, output, 5, "use --key (-k), --seed-file");
    assert!(env.is_empty());
}

#[test]
fn multisigning_requires_explicit_selection_and_uses_the_selected_key() {
    let env = fixture();
    let tx = transaction(GENESIS_ADDRESS);
    assert_unsigned_failure(
        &env,
        sign(&env, &["--multisign"], &tx),
        1,
        "explicit signer",
    );
    let output = sign(&env, &["--multisign", "-k", "alice-master"], &tx);
    output.assert_success();
    let signed = output.stdout_json();
    assert_eq!(signed["SigningPubKey"], "");
    let signer = &signed["Signers"][0]["Signer"];
    assert_eq!(signer["Account"], GENESIS_ADDRESS);
    let bytes =
        hex::decode(encode_for_multisigning(&signed, GENESIS_ADDRESS.into()).unwrap()).unwrap();
    assert!(is_valid_message(
        &bytes,
        signer["TxnSignature"].as_str().unwrap(),
        signer["SigningPubKey"].as_str().unwrap()
    ));
}

#[test]
fn streams_infer_one_account_and_refuse_mixed_accounts_before_any_signature() {
    let env = fixture();
    let tx = transaction(GENESIS_ADDRESS);
    let mixed = format!("{tx}\n{}\n", transaction(DESTINATION));
    assert_unsigned_failure(
        &env,
        env.run_with_stdin(&["tx", "sign", "-y"], mixed.as_bytes()),
        1,
        "one Account throughout the stream",
    );
    let same = format!("{tx}\n{tx}\n");
    let output = env.run_with_stdin(&["tx", "sign", "-y"], same.as_bytes());
    output.assert_success();
    assert_eq!(output.stdout.lines().count(), 2);
    for line in output.stdout.lines() {
        verify(&serde_json::from_str(line).unwrap());
    }
    assert_eq!(output.stderr.matches("signing as").count(), 1);
    assert_eq!(
        std::fs::read_to_string(env.data_dir().join("signing.log"))
            .unwrap()
            .lines()
            .count(),
        2
    );
    env.run_with_stdin(
        &["tx", "sign", "-y", "-k", "alice-master"],
        mixed.as_bytes(),
    )
    .assert_success();
}

#[test]
fn ed25519_sole_key_is_inferred_too() {
    let env = TestEnv::new().env("XRPL_PASSPHRASE", "ed25519 test");
    let generated = env.run(&["key", "generate", "ed", "--algorithm", "ed25519"]);
    generated.assert_success();
    env.run(&["account", "add", "ed-account", "--key", "ed"])
        .assert_success();
    let address = env.run(&["key", "show", "ed", "--address"]);
    address.assert_success();
    let output = sign(&env, &[], &transaction(address.stdout.trim()));
    output.assert_success();
    verify(&output.stdout_json());
    assert!(output.stdout_json()["SigningPubKey"]
        .as_str()
        .unwrap()
        .starts_with("ED"));
}
