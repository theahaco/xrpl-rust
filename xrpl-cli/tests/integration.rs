//! CLI integration tests.
//!
//! These drive the built `xrpl` binary as a subprocess.
//! Most tests run against Docker standalone rippled (localhost:5005).
//! Genesis account is used for testing since it has unlimited XRP in standalone mode.
//! Faucet tests use the public testnet since Docker standalone doesn't have a faucet.

#[cfg(feature = "integration")]
mod cli_tests {
    use std::io::{self};
    use std::process::{Command, Stdio};
    use std::str;

    /// Test-specific constants
    mod constants {
        // Docker standalone JSON-RPC endpoint (see docker-compose.yml).
        pub const TEST_URL: &str = "http://localhost:5005";

        // Public testnet for faucet tests only
        pub const TESTNET_URL: &str = "https://s.altnet.rippletest.net:51234";

        // Genesis account (has unlimited XRP in standalone mode)
        pub const TEST_SEED: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";
        pub const TEST_CLASSIC_ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh"; // Genesis account
        pub const TEST_X_ADDRESS: &str = "X7AcgcsBL6XDcUb289X4mJ8djcdyKaB5hJDWMArnXr61cqZ";

        // Error strings a test may CHOOSE to tolerate, because it talks to a
        // flaky public endpoint. This is no longer an ambient default: every
        // call site that wants it now names it, so a new assertion is strict
        // unless it opts out on purpose.
        //
        // The two reactor strings ("there is no reactor running", "must be
        // called from the context of a Tokio 1.x runtime") are deliberately
        // gone. A "Cannot start a runtime from within a runtime" panic is a bug
        // in every command family (#11), and a suite configured to treat it as
        // expected noise is worse than no suite.
        pub const TOLERATED_PUBLIC_ENDPOINT_ERRORS: &[&str] =
            &["expected value", "network", "connection", "timeout"];
    }

    /// Helper function to run the CLI with arguments and capture output
    fn run_cli_command(args: &[&str]) -> Result<String, io::Error> {
        let cmd = Command::new(env!("CARGO_BIN_EXE_xrpl"))
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let output = cmd.wait_with_output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(io::Error::other(format!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Legacy assertion helper: tolerates the public-endpoint error strings on
    /// top of whatever the call site names.
    ///
    /// The tolerance is opt-in by virtue of calling *this* function. Every test
    /// here is scheduled for deletion with the commands it exercises, and new
    /// tests use the harness in `tests/common`, which is strict by
    /// construction — so nothing new can inherit this by accident.
    fn assert_cli_command(args: &[&str], expected_output: &str, known_errors: &[&str]) {
        let result = run_cli_command(args);

        match result {
            Ok(output) => {
                assert!(
                    output.contains(expected_output),
                    "Expected output to contain '{}', but got: {}",
                    expected_output,
                    output
                );
            }
            Err(err) => {
                let err_str = err.to_string();

                let matches_common_error = constants::TOLERATED_PUBLIC_ENDPOINT_ERRORS
                    .iter()
                    .any(|&e| err_str.contains(e));
                let matches_known_error = known_errors.iter().any(|&e| err_str.contains(e));

                assert!(
                    matches_common_error || matches_known_error,
                    "Unexpected error: {}. Expected one of: {:?} or common errors: {:?}",
                    err_str,
                    known_errors,
                    constants::TOLERATED_PUBLIC_ENDPOINT_ERRORS
                );
            }
        }
    }

    /// Helper for wallet-related command tests
    fn assert_wallet_output(output: &str) {
        assert!(output.contains("classic_address"));
        assert!(output.contains("public_key"));
        assert!(output.contains("private_key"));
    }

    /// Helper to create address-related command arguments
    fn address_command_args<'a>(
        command_group: &'a str,
        subcommand: &'a str,
        address: &'a str,
        url: Option<&'a str>,
        limit: Option<u32>,
    ) -> Vec<&'a str> {
        let mut args = vec![command_group, subcommand, "--address", address];

        if let Some(url_str) = url {
            args.push("--url");
            args.push(url_str);
        }

        if let Some(limit_val) = limit {
            args.push("--limit");
            args.push(match limit_val {
                5 => "5",
                10 => "10",
                _ => "10",
            });
        }

        args
    }

    // ===== WALLET OPERATIONS TESTS =====

    /// The contract every migrated wallet command follows: exactly one JSON
    /// object on stdout and nothing else, so a script can `jq` it.
    fn wallet_json(output: &str) -> serde_json::Value {
        let value: serde_json::Value =
            serde_json::from_str(output.trim()).unwrap_or_else(|error| {
                panic!("stdout should be one JSON object, got {output:?}: {error}")
            });

        assert!(
            value["classic_address"]
                .as_str()
                .is_some_and(|address| address.starts_with('r')),
            "classic_address should be an r-address: {value}"
        );
        assert!(
            !value["public_key"].as_str().unwrap_or_default().is_empty(),
            "public_key should be present: {value}"
        );
        assert!(value["algorithm"].is_string(), "algorithm missing: {value}");

        value
    }

    #[test]
    fn test_generate_wallet() {
        let output = run_cli_command(&["wallet", "generate"])
            .expect("Failed to run wallet generate command");

        let value = wallet_json(&output);

        // Without --show-secret the seed is generated and thrown away. stdout
        // must not carry it, or redirecting this to a file leaks a key the user
        // did not ask to keep.
        assert!(
            value.get("seed").is_none(),
            "seed must not appear without --show-secret: {value}"
        );
    }

    #[test]
    fn test_generate_wallet_show_secret() {
        let output = run_cli_command(&["wallet", "generate", "--show-secret"])
            .expect("Failed to run wallet generate --show-secret");

        let value = wallet_json(&output);
        assert!(
            value["seed"]
                .as_str()
                .is_some_and(|seed| seed.starts_with('s')),
            "--show-secret should print a base58 family seed: {value}"
        );
    }

    /// Write a seed into a private file and hand back its path.
    ///
    /// The directory is 0700 and the file 0600, which is what `--seed-file`
    /// requires: a 0600 file inside a world-readable directory is still one
    /// anyone can find.
    fn private_seed_file(seed: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("seed");
        std::fs::write(&path, format!("{seed}\n")).expect("write seed");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
                .expect("chmod dir");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .expect("chmod file");
        }

        (dir, path)
    }

    #[test]
    fn test_wallet_from_seed() {
        // Through `--seed-file`, so no seed reaches `argv` and nothing in this
        // suite is visible in a process table while it runs.
        let (_dir, path) = private_seed_file(constants::TEST_SEED);

        let output =
            run_cli_command(&["wallet", "from-seed", "--seed-file", path.to_str().unwrap()])
                .expect("Failed to run wallet from-seed command");

        // TEST_SEED is the standalone genesis seed, so the address it derives is
        // known — assert the value rather than merely that a field exists.
        assert_eq!(
            wallet_json(&output)["classic_address"].as_str(),
            Some(constants::TEST_CLASSIC_ADDRESS)
        );
    }

    #[test]
    fn test_a_seed_never_reaches_argv_in_this_suite() {
        // The suite used to pass the genesis seed as a command-line argument at
        // nine call sites. Deleting the commands that took one removed eight;
        // this asserts the last one stayed gone.
        let source = include_str!("integration.rs");

        assert!(
            !source.contains("\"--seed\","),
            "a test passes --seed in argv; use --seed-file or XRPL_SEED"
        );
    }

    /// `--save` and `--mnemonic` are kept as hidden flags purely so they fail
    /// with an explanation instead of clap's bare "unexpected argument". Assert
    /// the refusal, so neither can quietly come back.
    #[test]
    fn test_removed_wallet_generate_flags_explain_themselves() {
        for (flag, expected) in [
            ("--save", "does not write secrets to disk"),
            ("--mnemonic", "no derivation-path convention"),
        ] {
            let error = run_cli_command(&["wallet", "generate", flag])
                .expect_err("removed flags should exit non-zero");

            let message = error.to_string();
            assert!(
                message.contains(expected),
                "{flag} should explain itself, got: {message}"
            );
        }
    }

    /// This test uses the public testnet since Docker standalone doesn't have a faucet.
    #[test]
    fn test_generate_faucet_wallet() {
        // Faucet tests must use public testnet - Docker standalone has no faucet
        let result = run_cli_command(&["wallet", "faucet", "--url", constants::TESTNET_URL]);

        assert!(
            result.is_ok(),
            "Failed to generate faucet wallet: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(output.contains("Generated faucet wallet:"));
        assert_wallet_output(&output);
    }

    // ===== ACCOUNT QUERY TESTS =====

    #[test]
    fn test_account_info() {
        let args = address_command_args(
            "account",
            "info",
            constants::TEST_CLASSIC_ADDRESS, // Genesis account in standalone
            Some(constants::TEST_URL),
            None,
        );

        let result = run_cli_command(&args);

        // The account should exist since it's the genesis account
        assert!(
            result.is_ok(),
            "Failed to get account info: {:?}",
            result.err()
        );
        assert!(result.unwrap().contains("Account info:"));
    }

    #[test]
    fn test_get_fee() {
        assert_cli_command(
            &["server", "fee", "--url", constants::TEST_URL],
            "Current network fee:",
            &["Failed to get network fee"],
        );
    }

    #[test]
    fn test_account_tx() {
        let args = address_command_args(
            "account",
            "tx",
            constants::TEST_CLASSIC_ADDRESS,
            None,
            Some(5),
        );

        assert_cli_command(&args, "Account transactions:", &["Account not found"]);
    }

    #[test]
    fn test_server_info() {
        assert_cli_command(
            &["server", "info"],
            "Server info:",
            constants::TOLERATED_PUBLIC_ENDPOINT_ERRORS,
        );
    }

    #[test]
    fn test_ledger_data() {
        assert_cli_command(
            &["ledger", "data", "--limit", "5"],
            "Ledger data:",
            constants::TOLERATED_PUBLIC_ENDPOINT_ERRORS,
        );
    }

    #[test]
    fn test_account_objects() {
        let args = address_command_args(
            "account",
            "objects",
            constants::TEST_CLASSIC_ADDRESS,
            None,
            Some(5),
        );

        assert_cli_command(&args, "Account objects:", &["Account not found"]);
    }

    #[test]
    fn test_account_channels() {
        let args = address_command_args(
            "account",
            "channels",
            constants::TEST_CLASSIC_ADDRESS,
            Some(constants::TEST_URL),
            Some(5),
        );

        assert_cli_command(&args, "Account channels:", &["Account not found"]);
    }

    #[test]
    fn test_account_currencies() {
        let args = address_command_args(
            "account",
            "currencies",
            constants::TEST_CLASSIC_ADDRESS,
            Some(constants::TEST_URL),
            None,
        );

        assert_cli_command(&args, "Account currencies:", &["Account not found"]);
    }

    #[test]
    fn test_account_lines() {
        let args = address_command_args(
            "account",
            "lines",
            constants::TEST_CLASSIC_ADDRESS,
            Some(constants::TEST_URL),
            Some(5),
        );

        assert_cli_command(&args, "Account trust lines:", &["Account not found"]);
    }

    // ===== ADDRESS VALIDATION TESTS =====

    #[test]
    fn test_validate_classic_address() {
        let args = &[
            "wallet",
            "validate",
            "--address",
            constants::TEST_CLASSIC_ADDRESS,
        ];

        assert_cli_command(
            args,
            "Valid classic address:",
            &[], // This should never fail as it doesn't require network access
        );
    }

    #[test]
    fn test_validate_x_address() {
        let args = &["wallet", "validate", "--address", constants::TEST_X_ADDRESS];

        assert_cli_command(
            args,
            "Valid X-address:",
            &[], // This should never fail as it doesn't require network access
        );
    }

    #[test]
    fn test_validate_invalid_address() {
        let address = "not_a_valid_address";
        let result = run_cli_command(&["wallet", "validate", "--address", address]);

        // This test specifically expects an error
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid address:"));
    }
}
