use std::thread::sleep;
use std::time::Duration;

use xrpl::clients::websocket::WebSocketClient;
use xrpl::clients::{SingleExecutorMutex, XRPLSyncWebsocketIO};
use xrpl::models::requests::subscribe::{StreamParameter, Subscribe};

use crate::client;
use crate::commands::global::{NetworkArgs, DEFAULT_PAGINATION_LIMIT, DEFAULT_WEBSOCKET_URL};
use crate::error::Error;

/// Stream ledger events over a WebSocket connection.
///
/// This is the one command that needs the node's WebSocket port rather than
/// its JSON-RPC port, so `--network` resolves to a `ws(s)://` endpoint here.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Stream type to subscribe to (ledger, transactions, validations)
    #[arg(long, default_value = "ledger")]
    pub stream: String,

    /// Number of events to receive before exiting (0 for unlimited)
    #[arg(long, default_value_t = DEFAULT_PAGINATION_LIMIT)]
    pub limit: u32,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let stream = match self.stream.to_lowercase().as_str() {
            "ledger" => StreamParameter::Ledger,
            "transactions" => StreamParameter::Transactions,
            "validations" => StreamParameter::Validations,
            other => return Err(Error::other(format!("Unknown stream type: {other}"))),
        };

        let url = self.network.websocket_url(DEFAULT_WEBSOCKET_URL);
        let mut websocket: WebSocketClient<SingleExecutorMutex, _> =
            WebSocketClient::open(client::parse_url(&url)?)?;

        websocket.xrpl_send(Subscribe::builder().streams(vec![stream]).build().into())?;

        let mut count = 0;
        loop {
            if self.limit > 0 && count >= self.limit {
                break;
            }

            match websocket.xrpl_receive() {
                Ok(Some(response)) => {
                    println!("Received: {response:#?}");
                    count += 1;
                }
                // Nothing buffered yet; the socket is still open.
                Ok(None) => sleep(Duration::from_millis(100)),
                Err(error) => return Err(Error::Client(error)),
            }
        }

        Ok(())
    }
}
