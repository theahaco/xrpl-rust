//! The account and key records.

mod common;

use common::{assert_offline, TestEnv};
use serde_json::Value;

const DESTINATION: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";
const GENESIS_SEED: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";

/// Generate a key pair and record it, returning `(key id, address, public key)`.
fn recorded_key(env: &TestEnv, id: &str) -> (String, String) {
    let generated = env.run(&["wallet", "generate", "--show-secret"]);
    generated.assert_success();
    let wallet = generated.stdout_json();

    let public_key = wallet["public_key"]
        .as_str()
        .expect("public key")
        .to_string();
    let address = wallet["classic_address"]
        .as_str()
        .expect("address")
        .to_string();

    env.run(&["key", "add", id, "--public-key", &public_key])
        .assert_success();

    (address, public_key)
}

#[test]
fn test_an_account_round_trips_through_the_store() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");

    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        &address,
        "--network-id",
        "1",
        "--key",
        "alice-master",
        "--default-signer",
        "alice-master",
    ])
    .assert_success();

    let shown = env.run(&["account", "show", "alice", "--json"]);
    shown.assert_success();

    let record = shown.stdout_json();
    assert_eq!(record["address"], address);
    assert_eq!(record["network_id"], 1);
    assert_eq!(record["default_signer"], "alice-master");
    assert_eq!(record["watch_only"], false);
}

#[test]
fn test_listing_records_touches_no_network() {
    let env = TestEnv::new();
    recorded_key(&env, "alice-master");

    // The invariant that makes a credential-store backend usable later: an
    // address, a public key and an algorithm are answerable from one file, so
    // listing never prompts and never waits on a device.
    assert_offline(&env, &["account", "ls"]);
    assert_offline(&env, &["key", "ls"]);
    assert_offline(&env, &["key", "show", "alice-master"]);
}

#[test]
fn test_a_record_never_contains_key_material() {
    let env = TestEnv::new();
    recorded_key(&env, "alice-master");

    // Read the files directly rather than the command output: the invariant is
    // about what is written, not about what is printed.
    let keys_dir = env.data_dir().join("keys");
    let contents = std::fs::read_to_string(keys_dir.join("alice-master.toml")).expect("read");

    for forbidden in ["seed", "private", "secret", "mnemonic"] {
        assert!(
            !contents.to_lowercase().contains(forbidden),
            "a key record must not contain {forbidden}:\n{contents}"
        );
    }
}

#[cfg(unix)]
#[test]
fn test_records_are_written_privately() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnv::new();
    recorded_key(&env, "alice-master");

    let path = env.data_dir().join("keys/alice-master.toml");
    let mode = std::fs::metadata(&path).expect("stat").permissions().mode();
    let dir_mode = std::fs::metadata(path.parent().unwrap())
        .expect("stat")
        .permissions()
        .mode();

    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(dir_mode & 0o777, 0o700);
}

#[test]
fn test_an_alias_resolves_on_a_query_command() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");
    env.run(&["account", "add", "alice", "--address", &address])
        .assert_success();

    // Pointed at a closed port: resolution happens before anything is sent, so
    // the failure must be the connection rather than the name.
    let output = env.run(&[
        "account",
        "info",
        "--account",
        "alice",
        "--url",
        "http://127.0.0.1:1",
    ]);

    assert!(!output.success());
    assert!(
        !output.stderr.contains("No such account"),
        "the alias should have resolved: {}",
        output.stderr
    );
}

#[test]
fn test_the_old_address_flag_still_works() {
    let env = TestEnv::new();

    // Kept as a hidden alias for one minor so existing invocations do not break
    // on the rename.
    let output = env.run(&[
        "account",
        "info",
        "--address",
        DESTINATION,
        "--url",
        "http://127.0.0.1:1",
    ]);

    assert!(!output.success());
    assert!(
        !output.stderr.contains("unexpected argument"),
        "--address should still parse: {}",
        output.stderr
    );
}

#[test]
fn test_a_seed_passed_as_an_account_is_refused() {
    let env = TestEnv::new();

    // The conflation the three-noun split exists to prevent. At best this is
    // "no such account"; at worst it matches a stored alias and signs with the
    // wrong key.
    let output = env.run(&[
        "tx",
        "new",
        "payment",
        "--account",
        GENESIS_SEED,
        "--destination",
        DESTINATION,
        "--amount",
        "1000000",
    ]);

    output.assert_code(1);
    output.assert_stderr_contains("looks like a seed");
}

#[test]
fn test_an_inherited_mainnet_account_is_refused() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "main-key");

    // network 0 is mainnet.
    env.run(&[
        "account",
        "add",
        "main",
        "--address",
        &address,
        "--network-id",
        "0",
    ])
    .assert_success();
    env.run(&["account", "use", "main"]).assert_success();

    let output = env.run(&[
        "tx",
        "new",
        "payment",
        "--destination",
        DESTINATION,
        "--amount",
        "1000000",
    ]);

    output.assert_code(1);
    output.assert_stderr_contains("mainnet");
}

#[test]
fn test_an_inherited_account_is_announced() {
    let env = TestEnv::new().env("XRPL_ACCOUNT", "alice");
    let (address, _) = recorded_key(&env, "alice-master");
    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        &address,
        "--network-id",
        "1",
    ])
    .assert_success();

    let output = env.run(&[
        "tx",
        "new",
        "payment",
        "--destination",
        DESTINATION,
        "--amount",
        "1000000",
    ]);
    output.assert_success();

    // An inherited account is never invisible.
    output.assert_stderr_contains("resolved from XRPL_ACCOUNT");
    assert_eq!(output.stdout_json()["Account"], address);
}

#[test]
fn test_removing_a_key_an_account_uses_is_refused_by_default() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");
    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        &address,
        "--key",
        "alice-master",
    ])
    .assert_success();

    let refused = env.run(&["key", "rm", "alice-master"]);
    refused.assert_code(1);
    refused.assert_stderr_contains("still referenced by alice");

    env.run(&["key", "rm", "alice-master", "--force"])
        .assert_success();
}

#[test]
fn test_doctor_reports_a_dangling_key_reference() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");
    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        &address,
        "--key",
        "alice-master",
    ])
    .assert_success();
    env.run(&["key", "rm", "alice-master", "--force"])
        .assert_success();

    // Key ids are names people choose, so this is what a rename looks like from
    // the account's side — a first-class diagnosable state, not a crash.
    let report = env.run(&["account", "doctor", "--json"]);
    report.assert_success();

    let findings = report.stdout_json();
    let text = findings.to_string();
    assert!(text.contains("alice-master"), "{text}");
    assert!(text.contains("no record"), "{text}");
}

#[test]
fn test_adding_an_account_with_a_missing_key_is_refused() {
    let env = TestEnv::new();

    // A record that only fails at signing time is worse than one that fails now.
    let output = env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        DESTINATION,
        "--key",
        "not-recorded",
    ]);

    output.assert_code(4);
    output.assert_stderr_contains("No such key");
}

#[test]
fn test_a_name_cannot_escape_the_store() {
    let env = TestEnv::new();

    let output = env.run(&["account", "add", "../escape", "--address", DESTINATION]);
    output.assert_code(1);
    output.assert_stderr_contains("outside the store");
}

#[test]
fn test_an_unknown_account_exits_four() {
    let env = TestEnv::new();

    // Distinct from a usage error: a retry wrapper branches on it.
    env.run(&["account", "show", "nobody"]).assert_code(4);
    env.run(&["key", "show", "nobody"]).assert_code(4);
}

#[test]
fn test_a_watch_only_account_is_a_normal_state() {
    let env = TestEnv::new();

    let added = env.run(&["account", "add", "watcher", "--address", DESTINATION]);
    added.assert_success();
    added.assert_stderr_contains("watch-only");

    // It can be named and queried; it just cannot sign. That is what lets a
    // transaction be prepared on one machine and signed on another.
    let shown = env.run(&["account", "show", "watcher", "--json"]);
    shown.assert_success();
    assert_eq!(shown.stdout_json()["watch_only"], true);
}

#[test]
fn test_the_default_account_is_marked_and_clearable() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");
    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        &address,
        "--network-id",
        "1",
    ])
    .assert_success();

    env.run(&["account", "use", "alice"]).assert_success();
    let listed = env.run(&["account", "ls"]);
    listed.assert_success();
    assert!(
        listed.stdout.trim_start().starts_with('*'),
        "{}",
        listed.stdout
    );

    env.run(&["account", "use", "--clear"]).assert_success();
    let cleared = env.run(&["account", "ls"]);
    assert!(
        !cleared.stdout.trim_start().starts_with('*'),
        "{}",
        cleared.stdout
    );
}

#[test]
fn test_removing_an_account_leaves_its_keys_alone_by_default() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");
    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        &address,
        "--key",
        "alice-master",
    ])
    .assert_success();

    let removed = env.run(&["account", "rm", "alice"]);
    removed.assert_success();
    removed.assert_stderr_contains("no key records were removed");

    // Forgetting a record is not the same as destroying a key.
    env.run(&["key", "show", "alice-master"]).assert_success();
}

#[test]
fn test_key_ls_shows_the_public_key_alongside_the_id() {
    let env = TestEnv::new();
    let (_, public_key) = recorded_key(&env, "issuer");

    // Two people in a ceremony can both name a key `issuer` and mean different
    // keys, so the id alone is not enough to tell them apart.
    let listed = env.run(&["key", "ls", "--json"]);
    listed.assert_success();

    let rows: Value = listed.stdout_json();
    let row = &rows.as_array().expect("array")[0];
    assert_eq!(row["id"], "issuer");
    assert_eq!(row["public_key"], public_key);
}
