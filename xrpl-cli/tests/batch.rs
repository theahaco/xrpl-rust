//! Streams, tickets and XLS-56 `Batch` — the three things issue #23 asks the
//! `tx` pipeline to survive.
//!
//! What ties them together is that each is a claim about *more than one*
//! transaction at a time, and each has a failure mode a single-transaction test
//! cannot reach: a stream whose lines all carry the same sequence, a ticketed
//! pair that only works in one order, a batch that lands with one leg down and
//! reports success anyway.
//!
//! No seed appears in `argv` anywhere in this file, for the same reason as in
//! `tx.rs`: every signing stage writes one into a 0600 file under the test's
//! own temp directory and passes `--seed-file`.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};

use common::{
    assert_offline, blockchain_lock, standalone_available, CliOutput, TestEnv, GENESIS_ADDRESS,
    GENESIS_SEED, STANDALONE_URL,
};
use serde_json::Value;

/// Where the batch tests submit.
///
/// The same node as everything else: `.ci-config/xrpld.cfg` enables
/// `BatchV1_1`, so CI's container has XLS-56 and these tests exercise it rather
/// than skipping. They still skip on a node that lacks it, because a developer
/// may have a container that predates it — see [`skip_without_batch`].
const BATCH_URL: &str = STANDALONE_URL;

/// Enough drops to create an account.
///
/// The node's base reserve is 20 XRP. A payment below it funds nothing and
/// fails `tecNO_DST_INSUF_XRP` on the ledger, which is useful exactly once —
/// as the inner transaction that is supposed to fail.
const FUNDING_DROPS: &str = "25000000";

/// One drop: below the reserve, so a payment of it to an address that has never
/// existed cannot create the destination.
const BELOW_RESERVE_DROPS: &str = "1";

/// The batch account's own balance.
///
/// It pays two [`FUNDING_DROPS`] inner transactions out of this and still has to
/// stay above its own reserve afterwards, or the last inner fails for a reason
/// this test is not about.
const BATCH_ACCOUNT_DROPS: &str = "150000000";

/// The node's reference fee, which is 200 drops on this configuration rather
/// than the protocol default of 10.
///
/// `tx batch wrap` is offline and cannot ask, and the outer `Fee` it computes
/// from this is a signing field — so a wrong value here is `telINSUF_FEE_P`
/// after the signature has already been spent.
const BASE_FEE: &str = "200";

/// A destination for the offline tests, which never reach a ledger.
const DESTINATION: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";

macro_rules! require_standalone {
    ($env:expr) => {
        if !standalone_available($env) {
            eprintln!("skipping: no standalone node at {STANDALONE_URL}");
            return;
        }
    };
}

/// Skip the rest of a test when the node has no XLS-56.
///
/// Applied to the real submission rather than to a probe. A node without the
/// amendment answers `temDISABLED`, and that is the only reliable signal: the
/// `feature` RPC reports `enabled: false` for every amendment on a standalone
/// node — including the 85 that `.ci-config/xrpld.cfg` enables and that
/// demonstrably work — and a deliberately malformed probe never reaches the
/// amendment check, because local checks reject it first.
macro_rules! skip_without_batch {
    ($output:expr) => {
        if $output.stdout.contains("temDISABLED") || $output.stderr.contains("temDISABLED") {
            eprintln!("skipping: the node at {BATCH_URL} has no BatchV1_1");
            return;
        }
    };
}

#[test]
fn test_a_stream_of_payments_gets_consecutive_sequences_from_one_lookup() {
    let env = TestEnv::new();
    require_standalone!(&env);
    let _guard = blockchain_lock();

    let seed = seed_file(&env);

    // Four addresses that have never existed, so nothing here depends on what
    // an earlier run left behind.
    let mut stream = String::new();
    for _ in 0..4 {
        let destination = fresh_address(&env);
        stream.push_str(&payment(&env, GENESIS_ADDRESS, &destination, FUNDING_DROPS));
    }

    let filled = autofill(&env, STANDALONE_URL, &stream, &["--sequence-from-auto"]);
    let sequences: Vec<u64> = lines_of_json(&filled)
        .iter()
        .map(|line| line["Sequence"].as_u64().expect("autofill set a sequence"))
        .collect();

    // The round trips are not observable from out here; their consequence is.
    // A per-line `account_info` answers with the *same* sequence every time,
    // because none of the earlier lines has been submitted yet — so four
    // consecutive numbers is the evidence that one lookup seeded a local
    // counter, and only the first line could apply without it.
    assert_eq!(sequences.len(), 4);
    for pair in sequences.windows(2) {
        assert_eq!(
            pair[1],
            pair[0] + 1,
            "sequences are not consecutive: {sequences:?}"
        );
    }

    let signed = sign(&env, &seed, &filled);
    let submitted = submit(&env, STANDALONE_URL, &signed);
    submitted.assert_success();

    // And that consecutive was also *correct*: all four reach a validated
    // ledger, each with the number autofill handed it.
    let results = lines_of_json(&submitted.stdout);
    assert_eq!(results.len(), 4, "one validated result per line");

    for (result, sequence) in results.iter().zip(&sequences) {
        assert_eq!(result["meta"]["TransactionResult"], "tesSUCCESS");
        assert_eq!(result["validated"], true);
        assert_eq!(result["Sequence"], *sequence);
    }
}

#[test]
fn test_ticketed_transactions_validate_out_of_order() {
    let env = TestEnv::new();
    require_standalone!(&env);
    let _guard = blockchain_lock();

    let seed = seed_file(&env);

    let created = {
        let built = env.run(&[
            "tx",
            "new",
            "ticket-create",
            "--account",
            GENESIS_ADDRESS,
            "--ticket-count",
            "3",
        ]);
        built.assert_success();

        let filled = autofill(&env, STANDALONE_URL, &built.stdout, &[]);
        let signed = sign(&env, &seed, &filled);
        submit(&env, STANDALONE_URL, &signed)
    };
    created.assert_success();

    let tickets = created_tickets(&created.stdout_json());
    assert_eq!(tickets.len(), 3, "--ticket-count 3 made {tickets:?}");

    // One payment on the lowest ticket and one on the highest, prepared
    // separately because each carries its own single-ticket range.
    let low = {
        let destination = fresh_address(&env);
        let built = payment(&env, GENESIS_ADDRESS, &destination, FUNDING_DROPS);
        let range = format!("{first}:{first}", first = tickets[0]);
        let filled = autofill(&env, STANDALONE_URL, &built, &["--tickets", &range]);
        sign(&env, &seed, &filled)
    };
    let high = {
        let destination = fresh_address(&env);
        let built = payment(&env, GENESIS_ADDRESS, &destination, FUNDING_DROPS);
        let range = format!("{last}:{last}", last = tickets[2]);
        let filled = autofill(&env, STANDALONE_URL, &built, &["--tickets", &range]);
        sign(&env, &seed, &filled)
    };

    // Highest ticket first. Two sequence-numbered transactions submitted in
    // this order would be `tefPAST_SEQ` on the second; a ticket carries no
    // ordering at all, which is the whole reason to spend one.
    let submitted = submit(&env, STANDALONE_URL, &format!("{high}{low}"));
    submitted.assert_success();

    let results = lines_of_json(&submitted.stdout);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["TicketSequence"], tickets[2]);
    assert_eq!(results[1]["TicketSequence"], tickets[0]);

    for result in &results {
        assert_eq!(result["meta"]["TransactionResult"], "tesSUCCESS");
        assert_eq!(result["validated"], true);
        // Zero, and present. rippled answers `invalidTransaction: Field
        // 'Sequence' is required but missing` to a ticketed transaction with no
        // `Sequence` field at all.
        assert_eq!(result["Sequence"], 0);
    }
}

#[test]
fn test_a_batch_reports_each_inner_and_exits_three_when_one_fails() {
    let env = TestEnv::new();
    // Both guards, because they answer different questions: whether a node is
    // there at all, and whether the one that is there carries XLS-56. The
    // secure-store CI job runs this binary with no node running, and without
    // the first guard this test failed there rather than skipping.
    require_standalone!(&env);
    let _guard = blockchain_lock();

    let genesis = seed_file(&env);

    // The batch account is a fresh account rather than genesis, so the three
    // inner sequences belong to this test alone. The outer signature commits to
    // the inner IDs, and an ID is over the inner's sequence — so a sequence
    // consumed by anything else in between invalidates the whole batch.
    let (account, seed) = funded_account(&env, BATCH_URL, &genesis, BATCH_ACCOUNT_DROPS);

    let first = fresh_address(&env);
    let doomed = fresh_address(&env);
    let last = fresh_address(&env);

    let stream = format!(
        "{}{}{}",
        payment(&env, &account, &first, FUNDING_DROPS),
        payment(&env, &account, &doomed, BELOW_RESERVE_DROPS),
        payment(&env, &account, &last, FUNDING_DROPS),
    );

    // `--no-last-ledger-sequence`, because an inner is applied by the outer
    // transaction rather than scheduled on its own: the outer is what the
    // ledger accepts or expires.
    let filled = autofill(
        &env,
        BATCH_URL,
        &stream,
        &["--sequence-from-auto", "--no-last-ledger-sequence"],
    );

    // The outer consumes the account's next sequence and `wrap` renumbers the
    // inners from it, so the number autofill gave the first line is the outer's.
    let outer_sequence = lines_of_json(&filled)[0]["Sequence"]
        .as_u64()
        .expect("autofill set a sequence");

    let wrapped = env.run_with_stdin(
        &[
            "tx",
            "batch",
            "wrap",
            "--account",
            &account,
            // Every inner is attempted whatever the others do, which is what
            // makes one failure among three observable at all.
            "--flag",
            "tfIndependent",
            "--sequence",
            &outer_sequence.to_string(),
            "--base-fee",
            BASE_FEE,
        ],
        filled.as_bytes(),
    );
    wrapped.assert_success();

    let signed = sign(&env, &seed, &wrapped.stdout);
    let submitted = submit(&env, BATCH_URL, &signed);
    skip_without_batch!(submitted);

    // The whole point of the command: the batch landed, one leg failed, and the
    // exit code says so even though the outer transaction succeeded.
    submitted.assert_code(3);

    let lines = lines_of_json(&submitted.stdout);
    assert_eq!(lines.len(), 4, "the outer result and one line per inner");

    assert_eq!(lines[0]["TransactionType"], "Batch");
    assert_eq!(lines[0]["meta"]["TransactionResult"], "tesSUCCESS");
    assert_eq!(lines[0]["validated"], true);

    // Inner transactions apply as separate transactions in the same ledger,
    // each with its own metadata, so each has a result of its own to report.
    let inners = &lines[1..];
    for (index, inner) in inners.iter().enumerate() {
        assert_eq!(inner["index"], index);
        assert!(
            inner["hash"].as_str().is_some_and(|hash| hash.len() == 64),
            "inner {index} has no transaction ID: {inner}"
        );
    }

    assert_eq!(inners[0]["result"], "tesSUCCESS");
    assert_eq!(inners[1]["result"], "tecNO_DST_INSUF_XRP");
    assert_eq!(inners[2]["result"], "tesSUCCESS");

    // Exit 3 usually means nothing landed. Here it means the opposite, and a
    // retry wrapper that cannot tell the two apart will resubmit a batch whose
    // sequences are already spent.
    submitted.assert_stderr_contains("tecNO_DST_INSUF_XRP");
    submitted.assert_stderr_contains("Exit 3");
    submitted.assert_stderr_contains("nothing happened");
}

#[test]
fn test_batch_wrap_is_offline_and_guards_its_input() {
    let env = TestEnv::new();

    // `wrap` has no `--url` at all, so clap rejecting the flag is stronger
    // evidence than a refused connection: the command has no way to name a node.
    let pair = format!(
        "[{},{}]",
        inner(r#","Sequence":1"#),
        inner(r#","Sequence":2"#)
    );
    assert_offline(
        &env,
        &[
            "tx",
            "batch",
            "wrap",
            "--account",
            GENESIS_ADDRESS,
            "--flag",
            "tfIndependent",
            pair.as_str(),
        ],
    );
    assert!(env.is_empty(), "folding a batch wrote into the state dir");

    // A batch is 2 to 8 inner transactions. One is not a batch, and nine is
    // past rippled's `maxBatchTxCount` — which it reports as `temMALFORMED`
    // after the signature is spent.
    let one = wrap(&env, &inner(r#","Sequence":1"#));
    one.assert_code(1);
    one.assert_stderr_contains("a Batch holds 2 to 8 inner transactions, got 1");

    let nine: String = (1..=9)
        .map(|n| inner(&format!(",\"Sequence\":{n}")))
        .collect();
    let nine = wrap(&env, &nine);
    nine.assert_code(1);
    nine.assert_stderr_contains("a Batch holds 2 to 8 inner transactions, got 9");

    // Exactly one authorization per inner. Neither is what an un-autofilled
    // transaction looks like; both is what a half-edited one looks like, and
    // the ledger answers `temMALFORMED` to each.
    let neither = wrap(&env, &format!("{}{}", inner(""), inner("")));
    neither.assert_code(1);
    neither.assert_stderr_contains("carries neither a non-zero Sequence nor a TicketSequence");

    let both = wrap(
        &env,
        &format!(
            "{}{}",
            inner(r#","Sequence":1,"TicketSequence":9"#),
            inner(r#","Sequence":2"#)
        ),
    );
    both.assert_code(1);
    both.assert_stderr_contains("carries both a non-zero Sequence and a TicketSequence");

    // Replay protection. The outer signature vouches for every inner ID it
    // commits to, so a batch is the easiest place to fold in a transaction
    // built for another chain.
    let crossed = wrap(
        &env,
        &format!(
            "{}{}",
            inner(r#","Sequence":1,"NetworkID":21338"#),
            inner(r#","Sequence":2,"NetworkID":1025"#)
        ),
    );
    crossed.assert_code(1);
    crossed.assert_stderr_contains("the batch crosses networks");

    // Every one of those was a refusal before anything was written, which is
    // what makes them safe to hit while assembling a ceremony.
    assert!(env.is_empty(), "a refused fold wrote into the state dir");
}

/// One inner transaction as a line, with `extra` appended inside the object.
///
/// Built as a literal rather than through `tx new`, because the offline test is
/// about what `wrap` refuses: the field combinations under test are exactly the
/// ones a well-formed pipeline would never produce.
fn inner(extra: &str) -> String {
    format!(
        "{{\"TransactionType\":\"Payment\",\"Account\":\"{GENESIS_ADDRESS}\",\
         \"Destination\":\"{DESTINATION}\",\"Amount\":\"1000000\",\"Fee\":\"200\"{extra}}}\n"
    )
}

/// Fold a stream, with no node involved and nothing asserted about the result.
fn wrap(env: &TestEnv, stream: &str) -> CliOutput {
    env.run_with_stdin(
        &[
            "tx",
            "batch",
            "wrap",
            "--account",
            GENESIS_ADDRESS,
            "--flag",
            "tfIndependent",
        ],
        stream.as_bytes(),
    )
}

/// Build one `Payment` line with `tx new`.
fn payment(env: &TestEnv, account: &str, destination: &str, drops: &str) -> String {
    let output = env.run(&[
        "tx",
        "new",
        "payment",
        "--account",
        account,
        "--destination",
        destination,
        "--amount",
        drops,
    ]);
    output.assert_success();

    output.stdout
}

/// The autofill stage, with whatever extra arguments the test is about.
fn autofill(env: &TestEnv, url: &str, stream: &str, extra: &[&str]) -> String {
    let mut args = vec!["tx", "autofill", "--url", url];
    args.extend_from_slice(extra);

    let output = env.run_with_stdin(&args, stream.as_bytes());
    output.assert_success();

    output.stdout
}

fn sign(env: &TestEnv, seed: &Path, stream: &str) -> String {
    let output = env.run_with_stdin(
        &[
            "tx",
            "sign",
            "--seed-file",
            seed.to_str().expect("utf-8 path"),
        ],
        stream.as_bytes(),
    );
    output.assert_success();

    output.stdout
}

/// Submit and wait for validation, closing ledgers to get it.
///
/// Returned rather than asserted on: half of what these tests are about is the
/// exit code of a submission that partly succeeded.
fn submit(env: &TestEnv, url: &str, stream: &str) -> CliOutput {
    env.run_with_stdin(
        &["tx", "submit", "--wait", "--accept-ledger", "--url", url],
        stream.as_bytes(),
    )
}

/// Every non-blank line of a stream, parsed.
fn lines_of_json(stream: &str) -> Vec<Value> {
    stream
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("line is not JSON ({error}): {line}"))
        })
        .collect()
}

/// The ticket sequences a `TicketCreate` actually created.
///
/// Read out of the metadata rather than computed from the account's sequence:
/// the ledger decides which numbers a ticket batch gets, and a test that
/// predicts them is testing its own arithmetic. Sorted, because the order of
/// `AffectedNodes` is the ledger's business and not part of any contract.
fn created_tickets(validated: &Value) -> Vec<u64> {
    let mut tickets: Vec<u64> = validated["meta"]["AffectedNodes"]
        .as_array()
        .expect("metadata carries affected nodes")
        .iter()
        .filter_map(|node| {
            let created = node.get("CreatedNode")?;
            (created["LedgerEntryType"] == "Ticket").then(|| {
                created["NewFields"]["TicketSequence"]
                    .as_u64()
                    .expect("a created Ticket carries its sequence")
            })
        })
        .collect();

    tickets.sort_unstable();
    tickets
}

/// Write a seed into a 0700 directory as a 0600 file.
///
/// Both halves matter, and `--seed-file` insists on both: a 0600 file inside a
/// world-readable directory is still one anyone can find.
fn write_seed(env: &TestEnv, name: &str, seed: &str) -> PathBuf {
    let dir = env.path().join("keys");
    std::fs::create_dir_all(&dir).expect("create key dir");

    let path = dir.join(format!("{name}.seed"));
    let mut file = std::fs::File::create(&path).expect("create seed file");
    writeln!(file, "{seed}").expect("write seed");
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod dir");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    }

    path
}

fn seed_file(env: &TestEnv) -> PathBuf {
    write_seed(env, "genesis", GENESIS_SEED)
}

fn fresh_address(env: &TestEnv) -> String {
    let generated = env.run(&["wallet", "generate"]);
    generated.assert_success();

    generated.stdout_json()["classic_address"]
        .as_str()
        .expect("address")
        .to_string()
}

/// A fresh account, funded from genesis on `url`, with a seed file of its own.
///
/// Generated through the CLI rather than the harness' `fund_from_genesis`, for
/// two reasons: that helper is pinned to [`STANDALONE_URL`], and the `Wallet` it
/// returns holds a zeroizing seed with no way to read it back out — so the
/// account could never sign for itself.
fn funded_account(env: &TestEnv, url: &str, genesis: &Path, drops: &str) -> (String, PathBuf) {
    let generated = env.run(&["wallet", "generate", "--show-secret"]);
    generated.assert_success();

    let value = generated.stdout_json();
    let address = value["classic_address"]
        .as_str()
        .expect("address")
        .to_string();
    let path = write_seed(env, &address, value["seed"].as_str().expect("seed"));

    let funding = payment(env, GENESIS_ADDRESS, &address, drops);
    let filled = autofill(env, url, &funding, &[]);
    let signed = sign(env, genesis, &filled);
    submit(env, url, &signed).assert_success();

    (address, path)
}
