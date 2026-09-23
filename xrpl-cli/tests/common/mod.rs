//! Shared harness for the CLI's integration tests.
//!
//! These tests drive the real `xrpl` binary as a subprocess, which has one
//! consequence worth stating up front: **an in-process test double is inert.**
//! A mock credential store or an in-memory signer installed in the test process
//! does nothing to the child. Any double reachable from here has to be selected
//! by environment variable or Cargo feature *on the child binary*.
//!
//! Lives at `tests/common/mod.rs` rather than `tests/common.rs`, because cargo
//! compiles every bare `tests/*.rs` as its own test binary.

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use serde_json::Value;

/// JSON-RPC endpoint of the standalone node CI runs (see docker-compose.yml).
pub const STANDALONE_URL: &str = "http://localhost:5005";

/// The genesis account's seed. **secp256k1**, not Ed25519.
pub const GENESIS_SEED: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";

/// The address `GENESIS_SEED` derives to under secp256k1.
pub const GENESIS_ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";

/// The flag naming an account on the ledger-query commands.
///
/// One constant rather than a spelling at each call site: it is `--address`
/// today and becomes `--account <alias|r-address>` when the account store lands.
/// What the tests assert is the round-trip, not the spelling.
pub const ACCOUNT_FLAG: &str = "--address";

/// How long any single CLI invocation may take before the harness gives up.
///
/// A command that blocks — on a prompt it should never have shown, on a ledger
/// that never closes — must surface as a named test failure with its partial
/// output, not as a job timeout half an hour later with nothing to read.
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// Serializes tests that mutate ledger state.
pub fn blockchain_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// What a CLI invocation produced.
///
/// stdout and stderr are kept apart and never folded together: stdout is the
/// machine artifact and stderr is everything a human reads, and a test that
/// concatenates them cannot tell the difference. The exit code is the raw
/// `i32`, with no named constants here — the binary's own table is the
/// authority, and a typed helper in the harness would only drift from it.
#[derive(Debug, Clone)]
pub struct CliOutput {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CliOutput {
    /// The exit code, failing the test if the process was killed by a signal.
    pub fn code(&self) -> i32 {
        self.code
            .unwrap_or_else(|| panic!("process terminated by signal\nstderr:\n{}", self.stderr))
    }

    pub fn success(&self) -> bool {
        self.code == Some(0)
    }

    /// Assert the command succeeded, showing both streams when it did not.
    pub fn assert_success(&self) -> &Self {
        assert!(
            self.success(),
            "expected exit 0, got {:?}\nstdout:\n{}\nstderr:\n{}",
            self.code,
            self.stdout,
            self.stderr
        );
        self
    }

    /// Assert a specific exit code.
    pub fn assert_code(&self, expected: i32) -> &Self {
        assert_eq!(
            self.code(),
            expected,
            "stdout:\n{}\nstderr:\n{}",
            self.stdout,
            self.stderr
        );
        self
    }

    /// Parse the whole of stdout as JSON.
    ///
    /// Deliberately parses the entire buffer rather than searching it: "nothing
    /// else on stdout" is the contract, and a `contains` check would pass for a
    /// command that also printed a friendly label.
    pub fn stdout_json(&self) -> Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|error| {
            panic!(
                "stdout is not exactly one JSON value ({error})\nstdout:\n{}\nstderr:\n{}",
                self.stdout, self.stderr
            )
        })
    }

    /// Assert that stderr mentions something, with stdout shown on failure.
    pub fn assert_stderr_contains(&self, needle: &str) -> &Self {
        assert!(
            self.stderr.contains(needle),
            "stderr does not contain {needle:?}\nstderr:\n{}\nstdout:\n{}",
            self.stderr,
            self.stdout
        );
        self
    }
}

/// An isolated environment for one test's CLI invocations.
///
/// Every child gets `XRPL_DATA_DIR`, `XRPL_CONFIG_DIR` and `HOME` pointed
/// inside a temporary directory. No command reads those yet — but every later
/// command does, and the alternative is a test suite that writes account
/// records into the developer's real home directory and enrols keys in their
/// real credential store. Four lines here, versus re-touching every helper.
pub struct TestEnv {
    dir: tempfile::TempDir,
    extra: HashMap<String, String>,
}

impl TestEnv {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        for sub in ["data", "config", "home"] {
            std::fs::create_dir_all(dir.path().join(sub)).expect("create test dirs");
        }

        Self {
            dir,
            extra: HashMap::new(),
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn data_dir(&self) -> PathBuf {
        self.dir.path().join("data")
    }

    pub fn config_dir(&self) -> PathBuf {
        self.dir.path().join("config")
    }

    /// Set an extra environment variable on every child this env spawns.
    ///
    /// This is the channel for secret input: stdin belongs to the transaction,
    /// so a seed or a passphrase cannot travel that way.
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.extra.insert(key.to_string(), value.to_string());
        self
    }

    /// Whether anything has been written under the temp directory's data and
    /// config trees. Used to prove a command that should write nothing wrote
    /// nothing.
    pub fn is_empty(&self) -> bool {
        [self.data_dir(), self.config_dir()]
            .iter()
            .all(|dir| is_dir_empty(dir))
    }

    /// Run the CLI with `args` and no stdin.
    pub fn run(&self, args: &[&str]) -> CliOutput {
        self.run_with_stdin(args, b"")
    }

    /// Run the CLI with `args`, writing `stdin` to the child.
    ///
    /// stdin is always piped and always closed, even when empty: a command that
    /// tries to prompt then fails immediately instead of blocking until the
    /// harness timeout.
    pub fn run_with_stdin(&self, args: &[&str], stdin: &[u8]) -> CliOutput {
        let mut command = Command::new(env!("CARGO_BIN_EXE_xrpl"));
        command
            .args(args)
            .env("XRPL_DATA_DIR", self.data_dir())
            .env("XRPL_CONFIG_DIR", self.config_dir())
            .env("HOME", self.dir.path().join("home"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &self.extra {
            command.env(key, value);
        }

        let mut child = command.spawn().expect("spawn xrpl");

        // Write on another thread. A single-threaded write to a full pipe
        // deadlocks against a child that is filling its own stdout.
        let mut handle = child.stdin.take().expect("piped stdin");
        let payload = stdin.to_vec();
        let writer = std::thread::spawn(move || {
            let _ = handle.write_all(&payload);
            // Dropping `handle` closes the pipe, so the child sees EOF.
        });

        let output = wait_with_timeout(child, COMMAND_TIMEOUT, args);
        let _ = writer.join();

        output
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Wait for a child, killing it if it outruns `timeout`.
fn wait_with_timeout(mut child: Child, timeout: Duration, args: &[&str]) -> CliOutput {
    let started = Instant::now();

    loop {
        match child.try_wait().expect("try_wait") {
            Some(_) => break,
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "`xrpl {}` did not finish within {timeout:?}. A command that blocks here is \
                     almost always waiting on a prompt it should never have shown.",
                    args.join(" ")
                );
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }

    let output = child
        .wait_with_output()
        .expect("collect child output after exit");

    CliOutput {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Run a command with no reachable node, and assert it succeeded anyway.
///
/// The offline stages of the pipeline, and the local record commands, must open
/// no socket at all. Pointing them at a closed port turns "this quietly reached
/// the network" into a failure rather than a slow test.
pub fn assert_offline(env: &TestEnv, args: &[&str]) -> CliOutput {
    // Port 1 is reserved and nothing listens on it.
    let mut with_url: Vec<&str> = args.to_vec();
    with_url.extend_from_slice(&["--url", "http://127.0.0.1:1"]);

    let output = env.run(&with_url);
    assert!(
        output.success(),
        "`xrpl {}` touched the network: it must work with no node reachable\nstderr:\n{}",
        args.join(" "),
        output.stderr
    );

    output
}

fn is_dir_empty(dir: &Path) -> bool {
    match std::fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_none(),
        // Absent counts as empty: nothing was written.
        Err(_) => true,
    }
}

/// Close a ledger on the standalone node.
///
/// A standalone node never closes one on its own, so anything waiting for
/// validation hangs until `LastLedgerSequence` passes unless this is called
/// between submissions.
pub fn ledger_accept(env: &TestEnv) -> CliOutput {
    let output = env.run(&["rpc", "ledger_accept", "--url", STANDALONE_URL]);
    output.assert_success();
    output
}

/// The current validated ledger index, read through `server_info`.
pub fn ledger_index(env: &TestEnv) -> u64 {
    let output = env.run(&["rpc", "server_info", "--url", STANDALONE_URL]);
    output.assert_success();

    output.stdout_json()["info"]["validated_ledger"]["seq"]
        .as_u64()
        .expect("server_info carries a validated ledger sequence")
}

/// Whether the standalone node is reachable.
///
/// Tests that need it skip rather than fail when it is not, so a developer
/// without the container running still gets a useful `cargo test`.
pub fn standalone_available(env: &TestEnv) -> bool {
    env.run(&["rpc", "server_info", "--url", STANDALONE_URL])
        .success()
}

/// Fund a fresh account from genesis and return its wallet.
///
/// Test-only, and deliberately not a preview of any CLI funding verb: it
/// submits a genesis `Payment` through the library, the same shape the root
/// crate's own helper uses.
///
/// The amount is bounded only by genesis' balance. It used to have to stay under
/// 4,294,967,295 drops, because `XRPAmount` validated drops through
/// `parse::<u32>()` and `validate()` runs before signing — so a larger top-up
/// failed in a way that looked like a node problem. Drops parse as `u64` now.
pub fn fund_from_genesis(drops: u64) -> xrpl::wallet::Wallet {
    use xrpl::asynch::clients::AsyncJsonRpcClient;
    use xrpl::asynch::transaction::sign_and_submit;
    use xrpl::models::transactions::payment::Payment;
    use xrpl::models::{Amount, XRPAmount};
    use xrpl::wallet::Wallet;

    let genesis = Wallet::new(GENESIS_SEED, 0).expect("genesis wallet");

    // Assert the derivation inside the helper. A plain `s…` seed resolves to
    // secp256k1, but deriving this one as Ed25519 is expressible now, and it
    // produces a valid-looking address that funds nothing — surfacing three
    // tests later as `tefBAD_AUTH`, nowhere near the mistake.
    assert_eq!(
        genesis.classic_address, GENESIS_ADDRESS,
        "the genesis seed must derive to {GENESIS_ADDRESS} under secp256k1"
    );

    let seed = xrpl::core::keypairs::generate_seed(None, None).expect("seed");
    let funded = Wallet::new(&seed, 0).expect("new wallet");

    let mut payment = Payment::builder(genesis.classic_address.clone())
        .amount(Amount::XRPAmount(XRPAmount::from(drops)))
        .destination(funded.classic_address.clone())
        .build();

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let result = runtime.block_on(async {
        let client =
            AsyncJsonRpcClient::connect(url::Url::parse(STANDALONE_URL).expect("standalone url"));
        sign_and_submit(&mut payment, &client, &genesis, true, true).await
    });

    let result = result.expect("funding payment");
    assert_eq!(
        result.engine_result, "tesSUCCESS",
        "funding payment was {}",
        result.engine_result
    );

    funded
}
