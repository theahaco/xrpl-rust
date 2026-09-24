//! The encrypted key store.

mod common;

use common::{standalone_available, TestEnv, GENESIS_ADDRESS, GENESIS_SEED, STANDALONE_URL};

/// Skip rather than fail when the standalone container is not running.
macro_rules! require_standalone {
    ($env:expr) => {
        if !standalone_available($env) {
            eprintln!("skipping: no standalone node at {STANDALONE_URL}");
            return;
        }
    };
}

const DESTINATION: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";

/// A test environment with a passphrase supplied the way a script would.
///
/// stdin carries the transaction, so a passphrase cannot travel that way. This
/// is the documented non-interactive path, and it is what the error names when
/// there is no terminal to prompt on.
fn env_with_passphrase() -> TestEnv {
    TestEnv::new().env("XRPL_PASSPHRASE", "a test passphrase")
}

/// Enrol the genesis seed, without it ever reaching `argv`.
fn enrol_genesis(env: &TestEnv, id: &str) {
    let path = private_seed_file(env);

    env.run(&["key", "add", id, "--seed-file", path.to_str().unwrap()])
        .assert_success();
}

/// A seed file, and a directory, that only its owner can read.
///
/// Both halves matter: a 0600 file inside a 0755 directory is still one anyone
/// can find, which is why the reader checks the directory too.
fn private_seed_file(env: &TestEnv) -> std::path::PathBuf {
    let dir = env.path().join("keys");
    std::fs::create_dir_all(&dir).expect("create");
    let path = dir.join("genesis.seed");
    std::fs::write(&path, format!("{GENESIS_SEED}\n")).expect("write");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod dir");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    }

    path
}

#[test]
fn test_an_enrolled_key_is_encrypted_at_rest() {
    let env = env_with_passphrase();
    enrol_genesis(&env, "genesis");

    let blob = std::fs::read_to_string(env.data_dir().join("secrets/genesis.age")).expect("read");

    // The property the whole design rests on: nothing this CLI writes contains
    // key material in the clear.
    assert!(!blob.contains(GENESIS_SEED), "{blob}");
    assert!(
        blob.starts_with("-----BEGIN AGE ENCRYPTED FILE-----"),
        "{blob}"
    );

    // And the record points at it rather than holding it.
    let record = std::fs::read_to_string(env.data_dir().join("keys/genesis.toml")).expect("read");
    assert!(!record.contains(GENESIS_SEED), "{record}");
    assert!(record.contains("encrypted-file"), "{record}");
}

#[test]
fn test_the_recorded_address_matches_the_seed() {
    let env = env_with_passphrase();
    enrol_genesis(&env, "genesis");

    let shown = env.run(&["key", "show", "genesis", "--json"]);
    shown.assert_success();

    assert_eq!(shown.stdout_json()["classic_address"], GENESIS_ADDRESS);
    assert_eq!(shown.stdout_json()["algorithm"], "secp256k1");
}

#[test]
fn test_generate_enrols_and_can_show_the_secret_once() {
    let env = env_with_passphrase();

    let generated = env.run(&["key", "generate", "alice", "--show-secret"]);
    generated.assert_success();

    let value = generated.stdout_json();
    let seed = value["seed"].as_str().expect("seed");
    assert!(seed.starts_with('s'), "{seed}");

    // The 16-byte family seed *is* the backup — there is no mnemonic
    // convention here — so a run without --show-secret says so rather than
    // leaving someone with a key they cannot record.
    let quiet = env.run(&["key", "generate", "bob"]);
    quiet.assert_success();
    assert!(quiet.stdout_json()["seed"].is_null());
    quiet.assert_stderr_contains("the seed was not printed");
}

#[test]
fn test_export_requires_saying_what_it_does() {
    let env = env_with_passphrase();
    enrol_genesis(&env, "genesis");

    let refused = env.run(&["key", "export", "genesis"]);
    refused.assert_code(1);
    refused.assert_stderr_contains("prints key material");

    let exported = env.run(&[
        "key",
        "export",
        "genesis",
        "--i-understand-this-prints-a-secret",
    ]);
    exported.assert_success();
    assert_eq!(exported.stdout_json()["seed"], GENESIS_SEED);
}

#[test]
fn test_a_wrong_passphrase_does_not_unlock() {
    let env = env_with_passphrase();
    enrol_genesis(&env, "genesis");

    let wrong = TestEnv::reusing(env.path()).env("XRPL_PASSPHRASE", "not the passphrase");
    let output = wrong.run(&[
        "key",
        "export",
        "genesis",
        "--i-understand-this-prints-a-secret",
    ]);

    output.assert_code(1);
    // One message for a wrong passphrase and a damaged file: telling them
    // apart tells an attacker which half they have.
    output.assert_stderr_contains("wrong passphrase, or the file is damaged");
}

#[test]
fn test_a_record_whose_secret_is_missing_is_unavailable_not_missing() {
    let env = env_with_passphrase();
    enrol_genesis(&env, "genesis");

    // What it looks like when records sync between machines and secrets do not.
    std::fs::remove_file(env.data_dir().join("secrets/genesis.age")).expect("remove");

    let output = env.run(&[
        "key",
        "export",
        "genesis",
        "--i-understand-this-prints-a-secret",
    ]);

    // Exit 6, not 4. The record is here; the secret is not, and the remedy is
    // different.
    output.assert_code(6);

    // And `doctor` says so in those terms.
    let report = env.run(&["account", "doctor", "--json"]);
    report.assert_success();
}

#[test]
fn test_signing_with_a_stored_key_needs_no_seed_in_argv() {
    let env = env_with_passphrase();
    require_standalone!(&env);
    let _guard = common::blockchain_lock();

    enrol_genesis(&env, "genesis");
    env.run(&[
        "account",
        "add",
        "genesis",
        "--address",
        GENESIS_ADDRESS,
        "--network-id",
        "0",
        "--key",
        "genesis",
        "--default-signer",
        "genesis",
    ])
    .assert_success();

    let built = env.run(&[
        "tx",
        "new",
        "payment",
        "--account",
        "genesis",
        "--destination",
        DESTINATION,
        "--amount",
        "400000000",
    ]);
    built.assert_success();

    let filled = env.run_with_stdin(
        &["tx", "autofill", "--url", STANDALONE_URL],
        built.stdout.as_bytes(),
    );
    filled.assert_success();

    let signed = env.run_with_stdin(
        &["tx", "sign", "--sign-with", "genesis"],
        filled.stdout.as_bytes(),
    );
    signed.assert_success();
    signed.assert_stderr_contains(GENESIS_ADDRESS);

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
    assert_eq!(
        submitted.stdout_json()["meta"]["TransactionResult"],
        "tesSUCCESS"
    );
}

#[test]
fn test_the_journal_records_the_signature_and_no_secret() {
    let env = env_with_passphrase();
    enrol_genesis(&env, "genesis");

    let built = env.run(&[
        "tx",
        "new",
        "payment",
        "--account",
        GENESIS_ADDRESS,
        "--destination",
        DESTINATION,
        "--amount",
        "1000000",
        "--field",
        "Fee=12",
        "--field",
        "Sequence=1",
    ]);
    built.assert_success();

    env.run_with_stdin(
        &["tx", "sign", "--sign-with", "genesis"],
        built.stdout.as_bytes(),
    )
    .assert_success();

    let journal = std::fs::read_to_string(env.data_dir().join("signing.log")).expect("read");

    // It is the only thing that answers "was my key used, and for what" after
    // the fact — the ledger only knows what was submitted.
    assert!(journal.contains("genesis"), "{journal}");
    assert!(journal.contains(GENESIS_ADDRESS), "{journal}");
    assert!(journal.contains("Single"), "{journal}");

    // Never the payload, never the secret.
    assert!(!journal.contains(GENESIS_SEED), "{journal}");
    assert!(!journal.contains(DESTINATION), "{journal}");
}

#[test]
fn test_the_ephemeral_backend_writes_no_journal() {
    let env = TestEnv::new();

    let path = private_seed_file(&env);

    let built = env.run(&[
        "tx",
        "new",
        "payment",
        "--account",
        GENESIS_ADDRESS,
        "--destination",
        DESTINATION,
        "--amount",
        "1000000",
        "--field",
        "Fee=12",
        "--field",
        "Sequence=1",
    ]);
    built.assert_success();

    env.run_with_stdin(
        &["tx", "sign", "--seed-file", path.to_str().unwrap()],
        built.stdout.as_bytes(),
    )
    .assert_success();

    // `tx sign --seed-file` stays a pure crypto stage with no filesystem
    // dependency, so it works on a read-only container, in a sandbox, and in CI.
    assert!(
        !env.data_dir().join("signing.log").exists(),
        "the ephemeral backend must not journal"
    );
}

#[test]
fn test_a_watch_only_key_cannot_sign() {
    let env = env_with_passphrase();

    let generated = env.run(&["wallet", "generate", "--show-secret"]);
    generated.assert_success();
    let public_key = generated.stdout_json()["public_key"]
        .as_str()
        .expect("public key")
        .to_string();

    env.run(&["key", "add", "watcher", "--public-key", &public_key])
        .assert_success();

    let built = env.run(&[
        "tx",
        "new",
        "payment",
        "--account",
        GENESIS_ADDRESS,
        "--destination",
        DESTINATION,
        "--amount",
        "1000000",
        "--field",
        "Fee=12",
        "--field",
        "Sequence=1",
    ]);
    built.assert_success();

    let output = env.run_with_stdin(
        &["tx", "sign", "--sign-with", "watcher"],
        built.stdout.as_bytes(),
    );

    output.assert_code(6);
    output.assert_stderr_contains("watch-only");
}

#[test]
fn test_key_add_with_nothing_to_record_says_so() {
    let env = env_with_passphrase();

    let output = env.run(&["key", "add", "nothing"]);
    output.assert_code(1);
    output.assert_stderr_contains("nothing to record");
}

// ---------------------------------------------------------------------------
// The OS credential store
// ---------------------------------------------------------------------------

/// What a binary built without the backend does with a record that needs it.
///
/// This is the case #14 is about: the `KeySource` variant is unconditional, so
/// a record written on a machine with the feature still parses here. Only
/// *using* it is refused, and the message names the missing backend rather than
/// looking like a corrupt file.
#[cfg(not(feature = "secure-store"))]
#[test]
fn test_a_secure_store_record_is_readable_without_the_backend() {
    let env = env_with_passphrase();

    std::fs::create_dir_all(env.data_dir().join("keys")).expect("create");
    std::fs::write(
        env.data_dir().join("keys/elsewhere.toml"),
        r#"version = 1
kind = "key"
public_key = "ED9434799226374926EDA3B54B1B461B4ABF7237962EAE18528FEA67595397FA32"
algorithm = "ed25519"
classic_address = "rLUEXYuLiQptky37CqLcm9USQpPiz5rkpD"
source = "secure-store"
entry = "elsewhere"
"#,
    )
    .expect("write");

    // It lists, and says where its secret is.
    let listed = env.run(&["key", "show", "elsewhere", "--json"]);
    listed.assert_success();
    assert_eq!(listed.stdout_json()["source"], "secure-store");

    // And it refuses at the point of use, naming the backend.
    let exported = env.run(&[
        "key",
        "export",
        "elsewhere",
        "--i-understand-this-prints-a-secret",
    ]);
    exported.assert_code(6);
    exported.assert_stderr_contains("secure-store");

    // `doctor` says the same thing in its own terms.
    env.run(&[
        "account",
        "add",
        "elsewhere",
        "--address",
        "rLUEXYuLiQptky37CqLcm9USQpPiz5rkpD",
        "--network-id",
        "0",
        "--key",
        "elsewhere",
    ])
    .assert_success();

    let report = env.run(&["account", "doctor", "elsewhere"]);
    report.assert_success();
    // The findings are the artifact, so they are on stdout.
    assert!(
        report
            .stdout
            .contains("built without the secure-store feature"),
        "stdout: {}",
        report.stdout
    );
}

#[test]
fn test_rm_leaves_the_secret_alone_unless_asked() {
    let env = env_with_passphrase();
    enrol_genesis(&env, "genesis");

    let blob = env.data_dir().join("secrets/genesis.age");
    assert!(blob.exists());

    env.run(&["key", "rm", "genesis"]).assert_success();

    // Forgetting a record is not destroying a key. The blob outlives it, and
    // `key add --seed-file` is not the only way back.
    assert!(blob.exists(), "rm must not destroy the secret");
}

#[test]
fn test_rm_delete_secret_destroys_it_and_says_so() {
    let env = env_with_passphrase();
    enrol_genesis(&env, "genesis");

    let blob = env.data_dir().join("secrets/genesis.age");
    let removed = env.run(&["key", "rm", "genesis", "--delete-secret"]);
    removed.assert_success();

    assert!(!blob.exists(), "--delete-secret must destroy the secret");
    // The 16-byte family seed is the only backup there is, so this says so
    // rather than reporting a tidy success.
    removed.assert_stderr_contains("that key is gone");
}

/// A real round trip through this machine's credential store.
///
/// `#[ignore]` on purpose: it writes an entry into the developer's own keychain
/// and, on macOS, raises an access dialog. CI builds the feature on all three
/// platforms — which is what catches the compile and link errors a
/// platform-specific backend actually produces — and real credential-store
/// verification is a manual pre-release step.
///
/// Run it with `cargo test -p xrpl-cli --features secure-store -- --ignored`.
#[cfg(feature = "secure-store")]
#[test]
#[ignore = "writes to the developer's real credential store"]
fn test_a_key_round_trips_through_the_credential_store() {
    let env = env_with_passphrase();
    let path = private_seed_file(&env);

    let added = env.run(&[
        "key",
        "add",
        "ceremony-test-key",
        "--seed-file",
        path.to_str().unwrap(),
        "--secure-store",
    ]);
    added.assert_success();
    added.assert_stderr_contains("secure-store");

    // Nothing on the filesystem: that is the whole difference this backend
    // makes, so it is what the test checks.
    assert!(
        !env.data_dir()
            .join("secrets/ceremony-test-key.age")
            .exists(),
        "the secure-store backend must not write a blob to disk"
    );

    let exported = env.run(&[
        "key",
        "export",
        "ceremony-test-key",
        "--i-understand-this-prints-a-secret",
    ]);
    exported.assert_success();
    assert_eq!(exported.stdout_json()["seed"], GENESIS_SEED);

    // Clean up after ourselves — this one lives outside the temp directory.
    env.run(&["key", "rm", "ceremony-test-key", "--delete-secret"])
        .assert_success();

    let gone = env.run(&[
        "key",
        "export",
        "ceremony-test-key",
        "--i-understand-this-prints-a-secret",
    ]);
    gone.assert_code(4);
}
