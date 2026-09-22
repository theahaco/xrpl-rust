use xrpl::clients::{
    websocket::{WebSocketClient, WebSocketOpen},
    SingleExecutorMutex, XRPLSyncWebsocketIO,
};
use xrpl::models::requests::subscribe::{StreamParameter, Subscribe};

fn main() {
    // open a websocket connection to a XRP Ledger node
    let mut websocket: WebSocketClient<SingleExecutorMutex, _> =
        WebSocketClient::open("wss://xrplcluster.com/".parse().unwrap()).unwrap();
    // subscribe to the ledger stream
    let subscribe = Subscribe::builder().streams(vec![StreamParameter::Ledger]).build();
    websocket.xrpl_send(subscribe.into()).unwrap();
    // listen for messages
    loop {
        let response = websocket.xrpl_receive().unwrap().unwrap();
        println!("subscription message: {:?}", response);
    }
}
