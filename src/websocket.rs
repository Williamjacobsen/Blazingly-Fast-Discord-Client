use futures_util::{SinkExt, StreamExt};
use std::{env, error::Error, time::Duration};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, Message},
};

pub fn send_heartbeat(
    transmitter: mpsc::UnboundedSender<Message>,
    heartbeat_interval: Option<u64>,
) -> Result<(), Box<dyn Error>> {
    let interval = heartbeat_interval.unwrap();
    tokio::spawn(async move {
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

pub async fn connect() -> Result<(), Box<dyn Error>> {
    let authorization_token =
        env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN environment variable not set");

    let gateway_url = "wss://gateway.discord.gg/?v=10&encoding=json";

    println!("Connecting to Discord Gateway...");
    let (ws_stream, _) = connect_async(gateway_url).await?;
    println!("Websocket connected!");

    let (write, mut read) = ws_stream.split();

    let (transmitter, mut receiver) = mpsc::unbounded_channel::<Message>();

    let writer = {
        tokio::spawn(async move {
            let mut write = write;
            while let Some(message) = receiver.recv().await {
                if let Err(e) = write.send(message).await {
                    eprintln!("websocket write error {}", e);
                    break;
                }
            }
            let _ = write.close().await;
        })
    };

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
                if let Some(heartbeat_interval) = Some(json["d"]["heartbeat_interval"].as_u64()) {
                    println!("Heartbeat interval: {}", heartbeat_interval.unwrap());

                    // TODO: Send identity payload with intents (opcode 2)

                    send_heartbeat(transmitter.clone(), heartbeat_interval)?;
                }
            }
        }
    }

    // This is just to keep the thread running...
    while let Some(message) = read.next().await {
        match message {
            Ok(msg) => {
                println!("Received: {:?}", msg);
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
        }
    }

    drop(transmitter);
    let _ = writer.await;

    /*let (mut write, mut read) = ws_stream.split();
    let mut heartbeat_interval: Option<u64> = None;

    let write_clone = std::sync::Arc::new(tokio::sync::Mutex::new(write));
    let write_heartbeat = write_clone.clone();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            if let Some(interval) = heartbeat_interval {
                let heartbeat = json!({"op": 1, "d": null});
                let mut w = write_heartbeat.lock().await;
                if let Err(e) = w.send(Message::Text(heartbeat.to_string().into())).await {
                    eprintln!("Heartbeat error: {}", e);
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                let data: serde_json::Value = serde_json::from_str(&text)?;

                match data["op"].as_u64() {
                    Some(10) => {
                        // HELLO - extract heartbeat interval
                        heartbeat_interval =
                            data["d"]["heartbeat_interval"].as_u64().map(|ms| ms / 1000);
                        println!("Received HELLO, heartbeat every {:?}s", heartbeat_interval);

                        // Send IDENTIFY
                        let identify = json!({
                            "op": 2,
                            "d": {
                                "token": authorization_token,
                                "properties": {
                                    "$os": std::env::consts::OS,
                                    "$browser": "rust-discord-client",
                                    "$device": "rust-discord-client"
                                },
                                "intents": 33281 // GUILDS + GUILD_MESSAGES + MESSAGE_CONTENT
                            }
                        });

                        let mut w = write_clone.lock().await;
                        w.send(Message::Text(identify.to_string().into())).await?;
                        println!("Sent IDENTIFY");
                    }
                    Some(0) => {
                        // DISPATCH - handle events
                        let event_name = data["t"].as_str().unwrap_or("UNKNOWN");
                        println!("Event: {}", event_name);

                        match event_name {
                            "READY" => {
                                println!("Bot is ready!");
                                if let Some(user) = data["d"]["user"].as_object() {
                                    println!(
                                        "Logged in as: {}#{}",
                                        user["username"].as_str().unwrap_or(""),
                                        user["discriminator"].as_str().unwrap_or("")
                                    );
                                }
                            }
                            "MESSAGE_CREATE" => {
                                if let Some(content) = data["d"]["content"].as_str() {
                                    let author = data["d"]["author"]["username"]
                                        .as_str()
                                        .unwrap_or("Unknown");
                                    println!("Message from {}: {}", author, content);
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(11) => {
                        // HEARTBEAT_ACK
                        println!("Heartbeat acknowledged");
                    }
                    _ => {
                        println!("Received: {}", text);
                    }
                }
            }
            Ok(Message::Close(frame)) => {
                println!("Connection closed: {:?}", frame);
                break;
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
            _ => {}
        }
    }*/

    Ok(())
}
