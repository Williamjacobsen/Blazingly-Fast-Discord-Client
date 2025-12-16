use dotenv::dotenv;
use serde::{Deserialize, Deserializer};
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
                            /* OP code: 0
                             * Incomming messages and updates are handled here.
                             */
                            0 => {
                                // Update `sequence` number
                                let sequence = payload
                                    .get("s")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or_default();

                                sequence_tracker.store(sequence, Ordering::Relaxed);
                                println!(
                                    "Sequence tracker: {}",
                                    sequence_tracker.load(Ordering::Relaxed)
                                );

                                // Handle incomming messages based on their `event_name`
                                if let Some(event_name) = payload.get("t").and_then(|v| v.as_str())
                                {
                                    match event_name {
                                        /*
                                         * Handles initial data requests by `Identify`.
                                         * This loads the following data into state:
                                         *      - Client profile information
                                         *      - Private channels
                                         *      - Guilds
                                         */
                                        "READY" => {
                                            /* Get client profile information:
                                             *      - id
                                             *      - username
                                             *      - global_name
                                             *      - avatar_hash
                                             */
                                            println!("Getting client profile information...");

                                            if let Some(user) = payload.pointer("/d/user") {
                                                let user: User =
                                                    serde_json::from_value(user.clone()).unwrap();
                                                println!("Logged in user info: {:?}", user);
                                            }

                                            /* Get private channels:
                                             *      - id
                                             *      - last_message_id
                                             *      - recipient ids
                                             * Also updates HashMap<id, User>
                                             */
                                            println!("Getting private channel information...");

                                            if let Some(private_channels) =
                                                payload.pointer("/d/private_channels")
                                            {
                                                if let Some(channels_array) =
                                                    private_channels.as_array()
                                                {
                                                    for channel in channels_array {
                                                        if let Some(recipients) = channel
                                                            .get("recipients")
                                                            .and_then(|r| r.as_array())
                                                        {
                                                            for recipient in recipients {
                                                                let user: User =
                                                                    serde_json::from_value(
                                                                        recipient.clone(),
                                                                    )
                                                                    .unwrap();
                                                                //users.insert(user.id, user);
                                                                println!("User: {:?}", user);
                                                            }
                                                        }

                                                        let channel: PrivateChannel =
                                                            serde_json::from_value(channel.clone())
                                                                .unwrap();

                                                        println!("Private channel: {:?}", channel);
                                                    }
                                                }
                                            }
                                        }

                                        other => {
                                            println!("Unhandled event: {}", other);
                                        }
                                    }
                                } else {
                                    println!("Dispatch event missing 't' field");
                                }
                            }

                            /* OP code: 10
                             * Handles Hello messages from Discord when establishing a websocket connection.
                             * Spawns a thread, for sending heartbeats.
                             * Each heartbeat must be sent every `heartbeat_interval`.
                             * Each heartbeat must contain the most up to date `sequence` number.
                             */
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
                                        tokio::time::sleep(Duration::from_millis(
                                            heartbeat_interval,
                                        ))
                                        .await;

                                        let heartbeat_payload = serde_json::json!({
                                            "op": 1,
                                            "d": sequence_tracker_clone.load(Ordering::Relaxed)
                                        });

                                        let _ = transmitter_clone.send(Message::Text(
                                            heartbeat_payload.to_string().into(),
                                        ));
                                    }
                                });
                            }

                            /* OP code: 11
                             * Handles the receiving of heartbeat ACK's (acknowledgements).
                             */
                            11 => {
                                println!("Received heartbeat ACK...")
                            }

                            /* OP code: _
                             * Any message with an OP code, that has not been handled, is handled here.
                             */
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

#[derive(Debug, Deserialize)]
struct User {
    #[serde(deserialize_with = "string_to_u64")]
    pub id: u64,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrivateChannel {
    #[serde(deserialize_with = "string_to_u64")]
    pub id: u64,

    #[serde(deserialize_with = "optional_string_to_u64")]
    pub last_message_id: Option<u64>,

    #[serde(rename = "recipients", deserialize_with = "deserialize_recipient_ids")]
    pub recipient_ids: Vec<u64>,
}

fn string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse::<u64>().map_err(serde::de::Error::custom)
}

fn optional_string_to_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) => s.parse::<u64>().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

fn deserialize_recipient_ids<'de, D>(deserializer: D) -> Result<Vec<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let recipients: Vec<User> = Vec::deserialize(deserializer)?;
    Ok(recipients.into_iter().map(|user| user.id).collect())
}
