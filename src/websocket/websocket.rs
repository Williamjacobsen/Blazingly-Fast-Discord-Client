use futures_util::StreamExt;
use std::{env, error::Error};
use tokio::sync::mpsc::{self};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::websocket::handle_connection::handle_connection;
use crate::websocket::handle_incomming_messages::handle_incomming_messages;
use crate::websocket::writer_task::writer_task;

// After connection is established and IDENTIFY is sent,
// it receives the s=1 (first sequence),
// it contains 4.2mb of data on everything,
// and by everything i mean everything,
// like the channel name of a server which the user is a member of,
// and the id of the last send message in that channel.
// After the necessary data has be loaded, discard the rest,
// when more data is needed (like client clicking on a guild/server),
// then send hello code again, and load the data that is now needed.

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
