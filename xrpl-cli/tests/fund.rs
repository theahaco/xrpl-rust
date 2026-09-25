//! Drive the actual CLI against local faucet and ledger HTTP fixtures.

mod common;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use common::{CliOutput, TestEnv, GENESIS_ADDRESS};
use serde_json::{json, Value};

struct Server {
    url: String,
    requests: Arc<Mutex<Vec<(String, Value)>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    fn new(handler: impl Fn(usize, &str, &Value) -> Option<(u16, Value)> + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("accept: {error}"),
                };
                // Accepted sockets can inherit the listener's nonblocking mode.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let (path, body) = read_request(&mut stream);
                let index = {
                    let mut requests = captured.lock().unwrap();
                    let index = requests.len();
                    requests.push((path.clone(), body.clone()));
                    index
                };
                if let Some((status, body)) = handler(index, &path, &body) {
                    let body = body.to_string();
                    let response = format!(
                        "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    // The timeout test may close its connection first.
                    let _ = stream.write_all(response.as_bytes());
                } else {
                    // Hold the response open to exercise an in-flight timeout.
                    while !stopped.load(Ordering::Relaxed) {
                        thread::sleep(Duration::from_millis(5));
                    }
                }
            }
        });
        Self {
            url,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn run(&self, env: &TestEnv, account: &str, extra: &[&str]) -> CliOutput {
        let node = format!("{}/rpc", self.url);
        let faucet = format!("{}/faucet", self.url);
        let mut args = vec![
            "account",
            "fund",
            account,
            "--url",
            &node,
            "--faucet-url",
            &faucet,
        ];
        args.extend_from_slice(extra);
        env.run(&args)
    }

    fn posts(&self) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path == "/faucet")
            .count()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Err(error) = self.thread.take().unwrap().join() {
            if !thread::panicking() {
                std::panic::resume_unwind(error);
            }
        }
    }
}

fn read_request(stream: &mut TcpStream) -> (String, Value) {
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    let header_end = loop {
        let count = stream.read(&mut chunk).expect("HTTP request");
        assert_ne!(count, 0, "incomplete HTTP headers");
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
    assert!(header.starts_with("POST "));
    let path = header.split_whitespace().nth(1).unwrap().to_string();
    let length = header
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .expect("content length");
    while bytes.len() < header_end + length {
        let count = stream.read(&mut chunk).expect("HTTP body");
        assert_ne!(count, 0, "incomplete HTTP body");
        bytes.extend_from_slice(&chunk[..count]);
    }
    (
        path,
        serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap(),
    )
}

fn account(balance: &str, ledger: u64) -> Value {
    json!({"result": {
        "status": "success", "validated": true, "ledger_index": ledger,
        "account_data": {"Account": GENESIS_ADDRESS, "Balance": balance}
    }})
}

fn not_found() -> Value {
    json!({"result": {"error": "actNotFound", "error_message": "Account not found."}})
}

fn successful_server(before: Option<&str>) -> Server {
    let before = before.map(str::to_string);
    Server::new(move |index, path, body| {
        if path == "/faucet" {
            assert_eq!(
                index, 1,
                "request funding only once, after the initial lookup"
            );
            assert_eq!(
                body,
                &json!({"destination": GENESIS_ADDRESS, "userAgent": "xrpl-cli"})
            );
            return Some((200, json!({"accepted": true})));
        }
        assert_eq!(path, "/rpc");
        assert_eq!(body["method"], "account_info");
        assert_eq!(body["params"][0]["account"], GENESIS_ADDRESS);
        assert_eq!(body["params"][0]["ledger_index"], "validated");
        Some((
            200,
            if index < 3 {
                before
                    .as_deref()
                    .map_or_else(not_found, |balance| account(balance, 10))
            } else {
                account("20000000", 11)
            },
        ))
    })
}

#[test]
fn funds_a_literal_address_after_validation_without_writing_a_store() {
    let env = TestEnv::new();
    let server = successful_server(None);
    let output = server.run(&env, GENESIS_ADDRESS, &["--json", "-q"]);
    output.assert_success();
    assert_eq!(
        output.stdout_json(),
        json!({
            "account": GENESIS_ADDRESS, "previous_balance": "0", "balance": "20000000",
            "funded": true, "validated": true, "ledger_index": 11
        })
    );
    assert_eq!(output.stdout.lines().count(), 1);
    assert!(output.stderr.is_empty());
    assert_eq!(server.posts(), 1);
    assert!(env.is_empty());
}

#[test]
fn tops_up_a_watch_only_alias_and_leaves_its_records_unchanged() {
    let env = TestEnv::new();
    env.run(&["account", "add", "alice", "--address", GENESIS_ADDRESS])
        .assert_success();
    let before = files(env.path());
    let server = successful_server(Some("10000000"));
    // The explicit URL wins over a named network, just as in other commands.
    let output = server.run(&env, "alice", &["--network", "mainnet"]);
    output.assert_success();
    assert_eq!(output.stdout_json()["previous_balance"], "10000000");
    assert!(output.stdout.lines().count() > 1);
    assert_eq!(server.posts(), 1);
    assert_eq!(files(env.path()), before);
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut result = BTreeMap::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(files(&path));
        } else {
            result.insert(path.clone(), std::fs::read(path).unwrap());
        }
    }
    result
}

#[test]
fn funding_does_not_resolve_a_keys_secret_source() {
    let env = TestEnv::new();
    env.run(&["account", "add", "alice", "--address", GENESIS_ADDRESS])
        .assert_success();
    let record = env.data_dir().join("accounts/alice.toml");
    let text = std::fs::read_to_string(&record).unwrap();
    assert!(text.contains("keys = []"));
    std::fs::write(
        &record,
        text.replace("keys = []", "keys = [\"missing-on-this-machine\"]"),
    )
    .unwrap();
    let before = files(env.path());
    let server = successful_server(None);
    server.run(&env, "alice", &[]).assert_success();
    assert_eq!(files(env.path()), before);
}

#[test]
fn a_malformed_record_is_rejected_before_network_io() {
    let env = TestEnv::new();
    env.run(&["account", "add", "bad", "--address", GENESIS_ADDRESS])
        .assert_success();
    let record = env.data_dir().join("accounts/bad.toml");
    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(record, text.replace(GENESIS_ADDRESS, "not-an-address")).unwrap();
    let output = env.run(&["account", "fund", "bad", "--network", "testnet"]);
    output
        .assert_code(1)
        .assert_stderr_contains("not a valid classic address");
    assert!(output.stdout.is_empty());
}

#[test]
fn endpoint_and_input_errors_do_not_reach_a_faucet() {
    let env = TestEnv::new().env("XRPL_NETWORK", "testnet");
    for (args, text) in [
        (vec![], "no network"),
        (vec!["--network", "mainnet"], "Mainnet has no"),
        (vec!["--network", "local"], "no built-in faucet"),
        (vec!["--url", "ws://127.0.0.1:1"], "HTTP or HTTPS"),
        (
            vec!["--network", "testnet", "--faucet-url", "file:///tmp/faucet"],
            "HTTP or HTTPS",
        ),
        (
            vec!["--network", "testnet", "--timeout", "0"],
            "invalid value",
        ),
    ] {
        let mut command = vec!["account", "fund", GENESIS_ADDRESS];
        command.extend(args);
        env.run(&command)
            .assert_code(1)
            .assert_stderr_contains(text);
    }
    for name in ["testnet", "devnet"] {
        env.run(&["account", "fund", "missing", "--network", name])
            .assert_code(4)
            .assert_stderr_contains("No such account");
    }
    env.run(&[
        "account",
        "fund",
        common::GENESIS_SEED,
        "--network",
        "testnet",
    ])
    .assert_code(1)
    .assert_stderr_contains("looks like a seed");
    assert!(env.is_empty());
}

#[test]
fn a_node_error_is_not_an_unfunded_account() {
    let server = Server::new(|_, _, _| {
        Some((
            200,
            json!({"result": {
                "error": "noPermission", "error_message": "fixture rejection"
            }}),
        ))
    });
    let output = server.run(&TestEnv::new(), GENESIS_ADDRESS, &[]);
    output.assert_code(2).assert_stderr_contains("noPermission");
    assert!(output.stdout.is_empty());
    assert_eq!(server.posts(), 0);
}

#[test]
fn faucet_rejection_is_a_network_error_and_is_not_retried() {
    let server = Server::new(|_, path, _| {
        Some(if path == "/faucet" {
            (429, json!({"error": "rate limited"}))
        } else {
            (200, not_found())
        })
    });
    let output = server.run(&TestEnv::new(), GENESIS_ADDRESS, &[]);
    output.assert_code(2).assert_stderr_contains("429");
    assert!(output.stdout.is_empty());
    assert_eq!(server.posts(), 1);
}

#[test]
fn malformed_or_unvalidated_balances_cannot_report_success() {
    let valid = account("20000000", 11);
    for (pointer, replacement) in [
        ("/result/validated", json!(false)),
        ("/result/validated", Value::Null),
        ("/result/account_data/Balance", json!("not a number")),
        ("/result/account_data/Balance", json!(20)),
        ("/result/account_data/Account", json!("another account")),
        ("/result/ledger_index", Value::Null),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(pointer).unwrap() = replacement;
        let server = Server::new(move |index, path, _| {
            Some((
                200,
                if index == 0 {
                    not_found()
                } else if path == "/faucet" {
                    json!({})
                } else {
                    bad.clone()
                },
            ))
        });
        let output = server.run(&TestEnv::new(), GENESIS_ADDRESS, &[]);
        output
            .assert_code(2)
            .assert_stderr_contains("funding is not confirmed");
        assert!(output.stdout.is_empty());
        assert_eq!(server.posts(), 1);
    }
}

#[test]
fn an_unchanged_balance_times_out_without_another_faucet_request() {
    let server = Server::new(|_, path, _| {
        Some((
            200,
            if path == "/faucet" {
                json!({})
            } else {
                account("10000000", 10)
            },
        ))
    });
    let output = server.run(&TestEnv::new(), GENESIS_ADDRESS, &["--timeout", "1"]);
    output
        .assert_code(2)
        .assert_stderr_contains("may still arrive");
    assert!(output.stdout.is_empty());
    assert_eq!(server.posts(), 1);
}

#[test]
fn the_deadline_also_bounds_a_hanging_faucet_post() {
    let server = Server::new(|_, path, _| {
        if path == "/faucet" {
            None
        } else {
            Some((200, not_found()))
        }
    });
    let start = Instant::now();
    let output = server.run(&TestEnv::new(), GENESIS_ADDRESS, &["--timeout", "1"]);
    output
        .assert_code(2)
        .assert_stderr_contains("not confirmed within 1s");
    assert!(start.elapsed() < Duration::from_secs(5));
    assert!(output.stdout.is_empty());
    assert_eq!(server.posts(), 1);
}

#[test]
fn the_deadline_also_bounds_the_initial_node_lookup() {
    let server = Server::new(|_, _, _| None);
    let output = server.run(&TestEnv::new(), GENESIS_ADDRESS, &["--timeout", "1"]);
    output
        .assert_code(2)
        .assert_stderr_contains("not confirmed within 1s");
    assert!(output.stdout.is_empty());
    assert_eq!(server.posts(), 0);
}
