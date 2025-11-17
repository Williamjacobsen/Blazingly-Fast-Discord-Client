use futures_util::StreamExt;
use std::{env, error::Error};
use tokio::sync::mpsc::{self};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::websocket::handle_connection::handle_connection;
use crate::websocket::handle_incomming_messages::handle_incomming_messages;
use crate::websocket::writer_task::writer_task;

/// Connects to discords websocket.
///
/// 1. Establishes a connection to the gateway.
/// 2. Receives "Hello" event (it contains heartbeat_interval), (opcode 10).
/// 3. Send identity (authorization_token) with intent (what you intent to received, like messages, guilds, etc), (opcode 2).
/// 4. Sends heartbeats event heartbeat_interval (opcode 1).
/// 5. Receives heartbeat ACK events (opcode 11). - NOT IMPLEMENTED
/// 6. Receives messages/updates from discord (opcode 0 && seq_num > 0).
pub async fn connect() -> Result<(), Box<dyn Error>> {
    let authorization_token =
        env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN environment variable not set");

    let gateway_url = "wss://gateway.discord.gg/?v=10&encoding=json";

    println!("Connecting to Discord Gateway...");
    let (ws_stream, _) = connect_async(gateway_url).await?;
    println!("Websocket connected!");

    let (write, mut read) = ws_stream.split();

    let (transmitter, receiver) = mpsc::unbounded_channel::<Message>();

    let writer = tokio::spawn(writer_task(write, receiver));

    handle_connection(&mut read, &authorization_token, transmitter.clone()).await?;

    handle_incomming_messages(&mut read).await?;

    drop(transmitter);
    let _ = writer.await;

    Ok(())
}
