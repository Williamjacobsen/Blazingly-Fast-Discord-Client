use std::{error::Error, sync::Arc, time::Duration};

use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::websocket::sequence_tracker::SequenceTracker;

/// Sends heartbeats through the unbounded Message channel, to the writer_task.
pub fn send_heartbeats(
    transmitter: mpsc::UnboundedSender<Message>,
    heartbeat_interval: u64,
    sequence_tracker: Arc<SequenceTracker>,
) -> Result<(), Box<dyn Error>> {
    let interval = heartbeat_interval;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(interval)).await;

        loop {
            let sequence = sequence_tracker.get();

            let heartbeat_payload = serde_json::json!({
                "op": 1,
                "d": sequence
            });

            if let Err(e) = transmitter.send(Message::Text(heartbeat_payload.to_string().into())) {
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
