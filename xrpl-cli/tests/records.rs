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

#[test]
fn test_an_account_takes_its_address_from_its_key() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");

    // An account's address derives from its master public key, so when that
    // key is one of the ones being recorded the address is already on disk and
    // asking for it again only means piping `key show` through `jq`.
    let added = assert_offline(&env, &["account", "add", "alice", "--key", "alice-master"]);
    added.assert_stderr_contains(&format!("address {address}, from key alice-master"));

    let shown = env.run(&["account", "show", "alice", "--json"]);
    shown.assert_success();
    assert_eq!(shown.stdout_json()["address"], address);
}

#[test]
fn test_an_explicit_address_wins_over_the_one_its_key_derives_to() {
    let env = TestEnv::new();
    let (key_address, _) = recorded_key(&env, "alice-regular");

    // A regular key and a signer-list member each derive to their own address
    // and not to the account's, which is the whole reason keys are a list
    // rather than a field. Deriving is a default, never a rule.
    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        DESTINATION,
        "--key",
        "alice-regular",
    ])
    .assert_success();

    let shown = env.run(&["account", "show", "alice", "--json"]);
    shown.assert_success();

    let record = shown.stdout_json();
    assert_eq!(record["address"], DESTINATION);
    assert_ne!(record["address"], key_address);
}

#[test]
fn test_keys_that_derive_to_different_addresses_are_refused() {
    let env = TestEnv::new();
    let (master, _) = recorded_key(&env, "alice-master");
    let (regular, _) = recorded_key(&env, "alice-regular");

    let output = env.run(&[
        "account",
        "add",
        "alice",
        "--key",
        "alice-master",
        "--key",
        "alice-regular",
    ]);

    // Which of the two this account is cannot be guessed, and guessing wrong
    // records an account that signs for someone else. Both candidates are
    // spelled out so the answer is readable off the refusal.
    output.assert_code(1);
    output.assert_stderr_contains(&format!("alice-master is {master}"));
    output.assert_stderr_contains(&format!("alice-regular is {regular}"));
    output.assert_stderr_contains("pass --address");
}

#[test]
fn test_an_account_with_neither_an_address_nor_a_key_is_refused() {
    let env = TestEnv::new();

    let output = env.run(&["account", "add", "alice"]);

    output.assert_code(1);
    output.assert_stderr_contains("nothing to record");
}

#[test]
fn test_key_show_prints_one_part_at_a_time() {
    let env = TestEnv::new();
    recorded_key(&env, "alice-master");

    let whole = env.run(&["key", "show", "alice-master", "--json"]);
    whole.assert_success();
    let whole = whole.stdout_json();

    for (flag, field) in [
        ("--address", "classic_address"),
        ("--public-key", "public_key"),
        ("--algorithm", "algorithm"),
        ("--source", "source"),
    ] {
        // Written the way the request was phrased — `key show --address alice`
        // — because that is the order a person types when the flag is the
        // question and the id is the subject.
        let part = assert_offline(&env, &["key", "show", flag, "alice-master"]);

        // Compared against the whole buffer rather than its trimmed contents:
        // a part is read with `$(...)`, so a label or a second line would each
        // have to be stripped back off by every caller.
        assert_eq!(
            part.stdout,
            format!("{}\n", whole[field].as_str().expect(field)),
            "{flag}"
        );
    }
}

#[test]
fn test_account_show_lists_its_keys_one_per_line() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");
    recorded_key(&env, "alice-regular");

    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        &address,
        "--key",
        "alice-master",
        "--key",
        "alice-regular",
    ])
    .assert_success();

    // One per line rather than the human block's comma-separated list, so a
    // `while read` loop needs no splitting.
    let shown = assert_offline(&env, &["account", "show", "alice", "--keys"]);
    assert_eq!(shown.stdout, "alice-master\nalice-regular\n");
}

#[test]
fn test_the_only_key_is_the_default_without_being_recorded_as_one() {
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

    // The effective signer, not only a recorded one: an account with a single
    // key has no `default_signer` field and that key is still the one to sign
    // with, which is the commonest record there is.
    let part = assert_offline(&env, &["account", "show", "alice", "--default-signer"]);
    assert_eq!(part.stdout, "alice-master\n");

    let human = env.run(&["account", "show", "alice"]);
    human.assert_success();
    assert!(
        human
            .stdout
            .contains("default signer  alice-master (the only key)"),
        "{}",
        human.stdout
    );
}

#[test]
fn test_the_default_signer_of_a_watch_only_account_is_unavailable() {
    let env = TestEnv::new();
    env.run(&["account", "add", "watcher", "--address", DESTINATION])
        .assert_success();

    // Not an empty answer: an unset tag is a field set to nothing, and this is
    // a question with no answer at all, so it fails rather than printing an
    // empty line a caller would have to tell apart from a key id.
    env.run(&["account", "show", "watcher", "--default-signer"])
        .assert_code(6);
}

#[test]
fn test_an_unset_field_prints_nothing_at_all() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");
    env.run(&["account", "add", "alice", "--address", &address])
        .assert_success();

    // `$(xrpl account show alice --tag)` has to come back empty. The word
    // "none" on stdout would be indistinguishable from a tag, and reading it
    // would mean every caller checking for that one string.
    let shown = assert_offline(&env, &["account", "show", "alice", "--tag"]);
    assert!(shown.stdout.is_empty(), "{}", shown.stdout);
    shown.assert_stderr_contains("no destination tag");
}

#[test]
fn test_naming_the_same_key_twice_is_refused() {
    let env = TestEnv::new();
    recorded_key(&env, "alice-master");

    // Accepted, this records an account with one key that cannot tell it has
    // one: the resolution that makes a lone key the default counts entries, so
    // the duplicate reappears as "several keys and no default" at signing time.
    let output = env.run(&[
        "account",
        "add",
        "alice",
        "--key",
        "alice-master",
        "--key",
        "alice-master",
    ]);

    output.assert_code(1);
    output.assert_stderr_contains("--key alice-master was given twice");
}

#[test]
fn test_an_ambiguous_default_signer_names_a_remedy_this_command_has() {
    let env = TestEnv::new();
    let (address, _) = recorded_key(&env, "alice-master");
    recorded_key(&env, "alice-regular");

    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        &address,
        "--key",
        "alice-master",
        "--key",
        "alice-regular",
    ])
    .assert_success();

    // `AccountRecord::signer_key` answers this case by naming `--key`,
    // which `tx sign` has and this command does not, so following the advice
    // from here is a dead end.
    let output = env.run(&["account", "show", "alice", "--default-signer"]);
    output.assert_code(1);
    output.assert_stderr_contains("--force --default-signer");
    assert!(!output.stderr.contains("--key"), "{}", output.stderr);

    // And the human block says so rather than omitting the line, which is the
    // record where a person most needs to be told to choose.
    let human = env.run(&["account", "show", "alice"]);
    human.assert_success();
    assert!(human.stdout.contains("several keys"), "{}", human.stdout);
}

#[test]
fn test_force_will_not_repoint_an_alias_at_a_key_of_its_own() {
    let env = TestEnv::new();
    let (regular, _) = recorded_key(&env, "alice-regular");

    // A regular key derives to its own address, never the account's, so this
    // account is deliberately recorded as something its key is not.
    env.run(&[
        "account",
        "add",
        "alice",
        "--address",
        DESTINATION,
        "--key",
        "alice-regular",
    ])
    .assert_success();

    // Adding a key is the obvious reason to re-run `add --force`. Until
    // `--address` became optional it had to be restated every time, so an alias
    // could not change identity by omission — and silently repointing it would
    // build every later transaction for an account nobody here controls.
    let output = env.run(&[
        "account",
        "add",
        "alice",
        "--force",
        "--key",
        "alice-regular",
    ]);

    output.assert_code(1);
    output.assert_stderr_contains(&format!("alice is recorded as {DESTINATION}"));
    output.assert_stderr_contains(&format!("derives to {regular}"));

    let shown = env.run(&["account", "show", "alice", "--address"]);
    shown.assert_success();
    assert_eq!(shown.stdout, format!("{DESTINATION}\n"));
}
