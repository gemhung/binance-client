mod models;

use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use structopt::StructOpt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::Message;
use tracing::*;
use url::Url;

static BINANCE_WS_API: &str = "wss://stream.binance.com:9443";

#[derive(Debug, StructOpt)]
#[structopt(name = "binance client", about = "An example of StructOpt usage.")]
struct Opt {
    #[structopt(short, long, default_value = "btcusdt")]
    symbol: String,

    /// 5, 10, 20
    #[structopt(short, long, default_value = "5")]
    level: usize,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();
    let opt = Opt::from_args();

    // checking level aruments
    anyhow::ensure!(matches!(opt.level, 5 | 10 | 20), "argument level hast to be either 5, 10, 20");

    let binance_url = format!(
        "{}/ws/{}@depth{}@100ms",
        BINANCE_WS_API,
        opt.symbol.to_lowercase(),
        opt.level,
    );

    let (socket, response) = connect_async(Url::parse(&binance_url)?).await?;

    info!("Connected to binance stream.");
    info!("HTTP status code: {}", response.status());
    info!("Response headers:");
    for (ref header, ref header_value) in response.headers() {
        info!("- {}: {:?}", header, header_value);
    }

    let (mut write, mut read) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // So read and write will be in different tokio tasks
    let handle = tokio::spawn(async move {
        while let Some(inner) = rx.recv().await {
            write.send(inner).await?;
        }

        Result::<_, anyhow::Error>::Ok(())
    });

    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(str) => {
                let parsed: models::DepthStreamData = serde_json::from_str(&str)?;
                for i in 0..parsed.asks.len() {
                    info!(
                        "{}. ask: {}, size: {}",
                        i, parsed.asks[i].price, parsed.asks[i].qty
                    );
                }
            }
            Message::Ping(p) => {
                info!("Ping message received! {:?}", String::from_utf8_lossy(&p));
                let pong = tungstenite::Message::Pong(vec![]);
                tx.send(pong)?;
            }
            Message::Pong(p) => info!("Pong received: {:?}", p),

            Message::Close(c) => {
                info!("Close received from binance : {:?}", c);
                break;
            }
            unexpected_msg => {
                info!(?unexpected_msg);
            }
        }
    }

    drop(read);
    handle.await??;

    Ok(())
}
