use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::{env, error::Error, time::Duration};
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::mpsc};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, Message},
};

// After connection is established and IDENTIFY is sent,
// it receives the s=1 (first sequence),
// it contains 4.2mb of data on everything,
// and by everything i mean everything,
// like the channel name of a server which the user is a member of,
// and the id of the last send message in that channel.
// After the necessary data has be loaded, discard the rest,
// when more data is needed (like client clicking on a guild/server),
// then send hello code again, and load the data that is now needed.

async fn save_pretty_json(path: &str, json: &Value) -> Result<(), std::io::Error> {
    let pretty = serde_json::to_string_pretty(json).unwrap_or_else(|_| json.to_string());
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(pretty.as_bytes()).await?;
    file.write_all(b"\n\n").await?;
    file.flush().await?;
    Ok(())
}

pub fn send_heartbeat(
    transmitter: mpsc::UnboundedSender<Message>,
    heartbeat_interval: Option<u64>,
) -> Result<(), Box<dyn Error>> {
    let interval = heartbeat_interval.unwrap();
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

                    let identify = json!({
                        "op": 2,
                        "d": {
                            "token": authorization_token,
                            "properties": {
                                "$os": std::env::consts::OS,
                                "$browser": "blazingly-rust-discord-client",
                                "$device": "blazingly-rust-discord-client"
                            },
                            "intents": 33281 // GUILDS + GUILD_MESSAGES + MESSAGE_CONTENT
                        }
                    });
                    transmitter.send(Message::Text(identify.to_string().into()))?;
                    println!("Sent IDENTIFY");

                    send_heartbeat(transmitter.clone(), heartbeat_interval)?;
                }
            }
        }
    }

    while let Some(message) = read.next().await {
        match message {
            Ok(message) => {
                if let Ok(text) = message.to_text() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        println!("Parsed JSON: {}", serde_json::to_string_pretty(&json)?);

                        if let (Some(op), Some(s), Some(t)) =
                            (json["op"].as_u64(), json["s"].as_u64(), json["t"].as_str())
                        {
                            println!("Opcode: {}", op);
                            println!("Sequence number: {}", s);
                            println!("Event type: {}", t);

                            if s == 1 && op == 0 {
                                // Load inital data:

                                // Get client username
                                if let Some(username) =
                                    json.pointer("/d/user/global_name").and_then(|v| v.as_str())
                                {
                                    println!("Username: {}", username);
                                }

                                // Get private channels (friends and groups)
                                if let Some(private_channels) = json
                                    .pointer("/d/private_channels")
                                    .and_then(|v| v.as_array())
                                {
                                    for private_channel in private_channels {
                                        if let Some(recipients) = private_channel
                                            .get("recipients")
                                            .and_then(|r| r.as_array())
                                        {
                                            if recipients.len() >= 2 {
                                                let group_name = private_channel
                                                    .get("name")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("<no group name>");
                                                println!("Group name: {}", group_name)
                                            }

                                            for recipient in recipients {
                                                let name = recipient
                                                    .get("global_name")
                                                    .and_then(|v| v.as_str())
                                                    .or_else(|| {
                                                        recipient
                                                            .get("username")
                                                            .and_then(|v| v.as_str())
                                                    })
                                                    .unwrap_or("<no name>");
                                                println!("Recipient: {}", name);
                                            }
                                        } else {
                                            println!(
                                                "No recipients or not an array: {}",
                                                private_channel
                                            );
                                        }
                                    }
                                }
                            } else if op == 0 {
                                if let Some(author_username) =
                                    json.pointer("/d/author/username").and_then(|v| v.as_str())
                                {
                                    println!("Author username: {}", author_username)
                                }
                            } else {
                                println!("Unhandled JSON: {}", serde_json::to_string_pretty(&json)?)
                            }

                            // temp:
                            break;
                        }
                    }
                }

                // Text(Utf8Bytes(b"{\"t\":\"MESSAGE_UPDATE\",\"s\":12,\"op\":0,\"d\":{\"type\":0,\"tts\":false,\"timestamp\":\"2025-11-15T16:57:35.201000+00:00\",\"pinned\":false,\"mentions\":[],\"mention_roles\":[],\"mention_everyone\":false,\"member\":{\"roles\":[\"854507461574262784\",\"904818008306905100\"],\"premium_since\":null,\"pending\":false,\"nick\":null,\"mute\":false,\"joined_at\":\"2021-11-01T19:43:43.978000+00:00\",\"flags\":0,\"deaf\":false,\"communication_disabled_until\":null,\"banner\":null,\"avatar\":null},\"id\":\"1439298298371379270\",\"flags\":0,\"embeds\":[{\"type\":\"rich\",\"title\":\"Guess the county\",\"image\":{\"width\":375,\"url\":\"https://gist.githubusercontent.com/GreenEyedBear/f4dfb4d911e284852edfde1b4614c27a/raw/d12547acef0b29ca8e0b1b83c9ea80f49de3c542/952677140443332749.png\",\"proxy_url\":\"https://images-ext-1.discordapp.net/external/-eGxu7A3hGzab0kak8MvR_MFM-jfJslbpCX5S2CnLTM/https/gist.githubusercontent.com/GreenEyedBear/f4dfb4d911e284852edfde1b4614c27a/raw/d12547acef0b29ca8e0b1b83c9ea80f49de3c542/952677140443332749.png\",\"placeholder_version\":1,\"placeholder\":\"+OeBCwIPNGvHCkYqDLGVAxASVHZTVmc=\",\"height\":722,\"flags\":0,\"content_type\":\"image/png\"},\"id\":\"1439298298371379271\",\"footer\":{\"text\":\"No image? Write `!pic`\"},\"content_scan_version\":2,\"color\":3918480}],\"edited_timestamp\":null,\"content\":\"\",\"components\":[{\"type\":1,\"id\":1,\"components\":[{\"type\":2,\"style\":2,\"label\":\"Skip question\",\"id\":2,\"custom_id\":\"efa52dd8ae9c20d25cc87a13f4ff6ee6\"}]}],\"channel_type\":0,\"channel_id\":\"1019630540049104926\",\"author\":{\"username\":\"MetaBot\",\"public_flags\":0,\"primary_guild\":null,\"id\":\"904794678686269480\",\"global_name\":null,\"display_name_styles\":null,\"discriminator\":\"1693\",\"collectibles\":null,\"clan\":null,\"bot\":true,\"avatar_decoration_data\":null,\"avatar\":\"a9de98041c9a0634282c9e814d1c9c5c\"},\"attachments\":[],\"guild_id\":\"854419081813164042\"}}"))
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
        }
    }

    drop(transmitter);
    let _ = writer.await;

    Ok(())
}
