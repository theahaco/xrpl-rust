//! `xrpl rpc` and the harness itself.
//!
//! Split from `integration.rs` so the strict assertions here cannot be relaxed
//! by that file's ambient error-swallowing, and so an OS matrix can select this
//! target on its own later.

mod common;

use common::{ledger_accept, ledger_index, standalone_available, TestEnv, STANDALONE_URL};

/// Skip rather than fail when the standalone container is not running, so
/// `cargo test` stays useful for a developer who has not started it.
macro_rules! require_standalone {
    ($env:expr) => {
        if !standalone_available($env) {
            eprintln!("skipping: no standalone node at {STANDALONE_URL}");
            return;
        }
    };
}

#[test]
fn test_param_parsing_is_a_usage_error_not_a_panic() {
    let env = TestEnv::new();

    // No node needed: this fails before any request is built.
    let output = env.run(&["rpc", "server_info", "--param", "nosign"]);

    assert!(!output.success());
    output.assert_stderr_contains("KEY=VALUE");
    assert!(
        output.stdout.is_empty(),
        "a failing command must not write to the machine channel: {}",
        output.stdout
    );
}

#[test]
fn test_a_reserved_param_is_refused() {
    let env = TestEnv::new();

    // `command` would collide with the field the request serializer emits, so
    // a caller could otherwise redirect the RPC by passing a parameter.
    let output = env.run(&["rpc", "server_info", "--param", "command=stop"]);

    assert!(!output.success());
    output.assert_stderr_contains("reserved");
}

#[test]
fn test_a_command_that_writes_nothing_writes_nothing() {
    let env = TestEnv::new();
    require_standalone!(&env);

    env.run(&["rpc", "server_info", "--url", STANDALONE_URL])
        .assert_success();

    // The isolation is only worth having if it is load-bearing. `xrpl rpc`
    // writes nothing today; this is what notices when something later does.
    assert!(
        env.is_empty(),
        "a command that should write nothing wrote into {:?}",
        env.path()
    );
}

#[test]
fn test_server_info_is_json_and_nothing_else_on_stdout() {
    let env = TestEnv::new();
    require_standalone!(&env);

    let output = env.run(&["rpc", "server_info", "--url", STANDALONE_URL]);
    output.assert_success();

    // Parsing the whole buffer, not searching it: "nothing else on stdout" is
    // the contract, and a `contains` check passes for a command that also
    // printed a label.
    let result = output.stdout_json();
    assert!(
        result["info"].is_object(),
        "server_info result should carry an `info` object: {result}"
    );
}

#[test]
fn test_ledger_accept_advances_the_ledger() {
    let env = TestEnv::new();
    require_standalone!(&env);
    let _guard = common::blockchain_lock();

    let before = ledger_index(&env);
    ledger_accept(&env);
    let after = ledger_index(&env);

    // This is the whole reason `xrpl rpc` exists: a standalone node never
    // closes a ledger on its own, so every later `--wait` would hang.
    assert!(
        after > before,
        "ledger_accept did not advance the ledger: {before} -> {after}"
    );
}

#[test]
fn test_a_json_param_reaches_the_node_as_json() {
    let env = TestEnv::new();
    require_standalone!(&env);

    // rippled rejects `"binary": "true"` with `invalidParams` and accepts
    // `"binary": true`. So this asserts the parameter arrived as a JSON boolean
    // and not as the string it was typed as on the command line.
    let output = env.run(&[
        "rpc",
        "ledger",
        "--param",
        "ledger_index=validated",
        "--param",
        "binary=true",
        "--url",
        STANDALONE_URL,
    ]);
    output.assert_success();

    let result = output.stdout_json();
    assert_ne!(
        result["error"], "invalidParams",
        "`binary=true` reached the node as a string, not a boolean: {result}"
    );
    assert!(result["error"].is_null(), "unexpected RPC error: {result}");
}

#[test]
fn test_the_harness_can_pipe_stdin() {
    let env = TestEnv::new();

    // Proves the harness can express the epic's pipelines. No command reads
    // stdin yet, so this asserts the mechanism rather than a behaviour: the
    // child receives the bytes, sees EOF, and exits rather than blocking.
    let output = env.run_with_stdin(&["--help"], b"{\"TransactionType\":\"Payment\"}\n");

    output.assert_success();
    assert!(
        output.stdout.contains("XRPL command line utility"),
        "stdout:\n{}",
        output.stdout
    );
}

#[test]
fn test_a_command_expecting_stdin_does_not_hang_on_an_empty_pipe() {
    let env = TestEnv::new();

    // stdin is always piped and always closed, so anything that tries to
    // prompt fails fast. Without that, this test would hit the harness
    // timeout instead of finishing in milliseconds.
    let output = env.run(&["rpc", "server_info", "--url", "http://127.0.0.1:1"]);

    // The node is unreachable, so this fails — the point is that it *returns*.
    assert!(!output.success());
}

#[test]
fn test_funding_round_trips_through_the_account_query() {
    let env = TestEnv::new();
    require_standalone!(&env);
    let _guard = common::blockchain_lock();

    let funded = common::fund_from_genesis(400_000_000);
    ledger_accept(&env);

    let output = env.run(&[
        "account",
        "info",
        common::ACCOUNT_FLAG,
        &funded.classic_address,
        "--url",
        STANDALONE_URL,
    ]);
    output.assert_success();

    // `account info` is still on the human channel, so this asserts the
    // round-trip rather than a JSON shape. #16 gives it `--json`.
    assert!(
        output.stdout.contains("400000000"),
        "expected the funded balance in the account info output:\n{}",
        output.stdout
    );
}
