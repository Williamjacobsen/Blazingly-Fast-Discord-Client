use futures_util::{stream::SplitStream, StreamExt};
use std::error::Error;
use tokio::{net::TcpStream, sync::mpsc::UnboundedSender};
use tokio_tungstenite::{
    tungstenite::{self, Message},
    MaybeTlsStream, WebSocketStream,
};

use crate::websocket::heartbeat::send_heartbeats;
use crate::websocket::load_initial_data::send_identity::send_identity;

pub async fn handle_connection(
    read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    authorization_token: &str,
    transmitter: UnboundedSender<Message>,
) -> Result<(), Box<dyn Error>> {
    if let Some(message) = read.next().await {
        println!("First message: {:?}", message);
        // Ok(Text(Utf8Bytes(b"{\"t\":null,\"s\":null,\"op\":10,\"d\":{\"heartbeat_interval\":41250,\"_trace\":[\"[\\\"gateway-prd-arm-us-east1-c-49x5\\\",{\\\"micros\\\":0.0}]\"]}}")))

        if let Ok(tungstenite::Message::Text(text)) = message {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(op) = json["op"].as_u64() {
                    if op != 10 {
                        return Err(format!("Expected opcode 10, got {}", op).into());
                    }
                }
                if let Some(heartbeat_interval) = json["d"]["heartbeat_interval"].as_u64() {
                    println!("Heartbeat interval: {}", heartbeat_interval);

                    send_identity(authorization_token, transmitter.clone()).await?;

                    send_heartbeats(transmitter.clone(), heartbeat_interval)?;
                }
            }
        }
    }

    Ok(())
}
