use std::{error::Error, time::Duration};

use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// Sends heartbeats through the unbounded Message channel, to the writer_task.
pub fn send_heartbeats(
    transmitter: mpsc::UnboundedSender<Message>,
    heartbeat_interval: u64,
) -> Result<(), Box<dyn Error>> {
    let interval = heartbeat_interval;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(interval)).await;

        loop {
            // TODO:
            // Send {"op": 1, "d": null}
            // Where "d" is initally null,
            // but later it is the last sequence number "s" (received from discord),
            // and that is according to discord documentation, even tough "s" is always "null" (i think).

            let heartbeat = serde_json::json!({
                "op": 1,
                "d": null
            });

            if let Err(e) = transmitter.send(Message::Text(heartbeat.to_string().into())) {
                eprintln!("Failed to send heartbeat: {}", e);
                break;
            }
            // Text(Utf8Bytes(b"{\"t\":null,\"s\":null,\"op\":11,\"d\":null}"))

            println!("Sent heartbeat");
            tokio::time::sleep(Duration::from_millis(interval)).await;
        }
    });

    Ok(())
}