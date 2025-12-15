use dotenv::dotenv;
use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{env, time::Duration};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    handle_websocket_connection().await?;

    run_app()?;

    Ok(())
}

async fn handle_websocket_connection() -> Result<(), Box<dyn Error>> {
    let authorization_token = env::var("DISCORD_TOKEN").expect("PANIC: No DISCORD_TOKEN set.");

    let gateway_url = "wss://gateway.discord.gg/?v=10&encoding=json";

    let (ws_stream, _) = connect_async(gateway_url).await?;

    let (mut write, mut read) = ws_stream.split();

    let (transmitter, mut receiver) = mpsc::unbounded_channel::<Message>();

    /*
     * Listen to `receiver` channel,
     * Sends message to discord through `write` websocket stream sink.
     */
    tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            if let Err(e) = write.send(msg).await {
                eprintln!("Write error: {}", e);
                break;
            }
        }
    });

    /*
     * Send `identify` with intent.
     * This is done, to later receive all initial data requested (by intents).
     */
    let identify = serde_json::json!({
        "op": 2,
        "d": {
            "token": authorization_token,
            "properties": {
                "$os": std::env::consts::OS,
                "$browser": "blazingly-rust-discord-client",
                "$device": "blazingly-rust-discord-client"
            },
            "intents": (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9) | (1 << 10) | (1 << 11) | (1 << 12) | (1 << 13) | (1 << 14) | (1 << 15) | (1 << 16) | (1 << 20) | (1 << 21) | (1 << 24) | (1 << 25)
        }
    });
    transmitter.send(Message::Text(identify.to_string().into()))?;
    println!("Sent IDENTIFY...");

    let sequence_tracker = Arc::new(AtomicU64::new(0));

    /*
     * Handles incomming messages on `read` websocket stream sink.
     */
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(message) => match message {
                    Message::Text(message) => {
                        //println!("{:?}", message);

                        let payload: serde_json::Value = serde_json::from_str(&message).unwrap();

                        let op = payload.get("op").and_then(|v| v.as_u64()).unwrap();

                        match op {
                            0 => {
                                let s = payload
                                    .get("s")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or_default();

                                // Handle initial data requests by `Identify`.
                                if s == 0 {
                                    // Do it here.

                                    continue;
                                }

                                sequence_tracker.store(s, Ordering::Relaxed);
                                println!(
                                    "Sequence tracker: {}",
                                    sequence_tracker.load(Ordering::Relaxed)
                                );

                                // Handle incomming messages.
                            }

                            10 => {
                                println!("Handling OP code 10...");

                                let heartbeat_interval = payload
                                    .pointer("/d/heartbeat_interval")
                                    .and_then(|v| v.as_u64())
                                    .unwrap();

                                let transmitter_clone = transmitter.clone();
                                let sequence_tracker_clone = sequence_tracker.clone();
                                tokio::spawn(async move {
                                    loop {
                                        let heartbeat_payload = serde_json::json!({
                                            "op": 1,
                                            "d": sequence_tracker_clone.load(Ordering::Relaxed)
                                        });

                                        let _ = transmitter_clone.send(Message::Text(
                                            heartbeat_payload.to_string().into(),
                                        ));

                                        tokio::time::sleep(Duration::from_millis(
                                            heartbeat_interval,
                                        ))
                                        .await;
                                    }
                                });
                            }

                            11 => {
                                println!("Received heartbeat ACK...")
                            }

                            other => {
                                eprintln!("Have not handled this case for OP code: {}", other)
                            }
                        }
                    }

                    _ => {}
                },
                Err(e) => {
                    eprintln!("Websocket error: {}", e);
                }
            }
        }
    });

    Ok(())
}

fn run_app() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    ui.run()?;

    Ok(())
}
