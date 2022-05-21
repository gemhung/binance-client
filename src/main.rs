
mod models;
use tungstenite::connect;
use url::Url;
use models::DepthStreamData;
use tracing::*;

static BINANCE_WS_API: &str = "wss://stream.binance.com:9443";
#[tokio::main]
async fn main() -> Result<(), anyhow::Error>{
    tracing_subscriber::fmt().init();
    let binance_url = format!("{}/ws/ethbtc@depth5@100ms", BINANCE_WS_API);
    let (mut socket, response) =
        connect(Url::parse(&binance_url).unwrap()).expect("Can't connect.");

    info!("Connected to binance stream.");
    info!("HTTP status code: {}", response.status());
    info!("Response headers:");
    for (ref header, ref header_value) in response.headers() {
        info!("- {}: {:?}", header, header_value);
    }

    loop {
        let msg = socket.read_message().expect("Error reading message");
        let msg = match msg {
            tungstenite::Message::Text(s) => s,
            tungstenite::Message::Ping(p) => {
                info!("Ping message received! {:?}", p);
                //let pong = tungstenite::protocol::frame::Frame::pong(vec![]);
                //let m2 = tungstenite::Message::Frame(pong);
                //socket.write_message(m2)?;
                // send_pong(&mut socket, p);
                continue;
            }
            tungstenite::Message::Pong(p) => {
                info!("Pong received: {:?}", p);
                continue;
            }
            _ => {
                error!("Error getting text: {:?}", msg);
                continue;
            }
        };

        let parsed: models::DepthStreamData = serde_json::from_str(&msg).expect("Can't parse");
        for i in 0..parsed.asks.len() {
            info!(
                "{}. ask: {}, size: {}",
                i, parsed.asks[i].price, parsed.asks[i].qty
            );
        }
    }
}