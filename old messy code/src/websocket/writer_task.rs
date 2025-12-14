use futures_util::{stream::SplitSink, SinkExt};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

/// writer_task listens on an unbounded receiver channel.
/// 
/// When a Message is send on that channel,
/// it sends Message through the websocket to discord.
pub async fn writer_task(
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut receiver: mpsc::UnboundedReceiver<Message>,
) {
    let mut write = write;
    while let Some(message) = receiver.recv().await {
        if let Err(e) = write.send(message).await {
            eprintln!("websocket write error {}", e);
            break;
        }
    }
    let _ = write.close().await;
}
