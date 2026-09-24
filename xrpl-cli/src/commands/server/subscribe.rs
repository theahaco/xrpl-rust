//! `xrpl server subscribe` — stream ledger events over a WebSocket.

use futures_util::StreamExt;
use xrpl::asynch::clients::{AsyncWebSocketClient, SingleExecutorMutex, XRPLAsyncWebsocketIO};
use xrpl::models::requests::subscribe::{StreamParameter, Subscribe};

use crate::commands::global::{NetworkArgs, DEFAULT_WEBSOCKET_URL};
use crate::error::Error;
use crate::{client, output};

/// How many events to receive before exiting, when no limit is given.
///
/// Its own constant rather than `DEFAULT_PAGINATION_LIMIT`: this counts events
/// off a stream, not rows in a page, and the two have no reason to move
/// together.
pub const DEFAULT_EVENT_LIMIT: u32 = 10;

/// Stream ledger events over a WebSocket connection.
///
/// This is the one command that needs the node's WebSocket port rather than
/// its JSON-RPC port, so `--network` resolves to a `ws(s)://` endpoint here.
///
/// # Output is NDJSON, with no flag to change it
///
/// One compact JSON object per line, per frame — **the node's own message**,
/// passed through rather than re-serialized. There is no `--json` here and no
/// indented default, for the same reason the `tx` stages have neither: a
/// stream's shape *is* its contract, and a reader consuming it line by line
/// cannot also be asked to sniff whether this particular run indented.
///
/// Passing the frame through is not laziness. A stream event is not a response
/// and does not parse as one, so reading through the typed envelope failed on
/// the second message with "Unexpected message type" — the subscribe
/// confirmation arrives as a response and every event after it does not. That
/// is #10's problem; here the fix is to stop pretending an event is a reply.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Stream type to subscribe to (ledger, transactions, validations)
    #[arg(long, default_value = "ledger")]
    pub stream: String,

    /// Number of events to receive before exiting (0 for unlimited)
    #[arg(long, default_value_t = DEFAULT_EVENT_LIMIT)]
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
        let runtime = client::runtime()?;

        runtime.block_on(async {
            let mut websocket: AsyncWebSocketClient<SingleExecutorMutex, _> =
                AsyncWebSocketClient::open(client::parse_url(&url)?).await?;

            websocket
                .xrpl_send(Subscribe::builder().streams(vec![stream]).build().into())
                .await?;

            output::note(format!("subscribed to {} at {url}", self.stream));

            let mut count = 0;
            while self.limit == 0 || count < self.limit {
                // Awaits the socket. The previous loop called a synchronous
                // receive that returned `None` when nothing was buffered and
                // slept 100ms, which burned a thread for the lifetime of the
                // subscription and added 100ms of latency to every event.
                match websocket.next().await {
                    Some(frame) => {
                        println!("{}", frame.map_err(Error::Client)?.trim());
                        count += 1;
                    }
                    // The stream ended. Not an error: a node closing the socket
                    // is how an unlimited subscription is meant to finish.
                    None => break,
                }
            }

            Ok::<(), Error>(())
        })
    }
}
