use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::{env, error::Error, time::Duration};
use tokio::sync::mpsc;
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
                let log_entry = format!("Received: {:?}", message);
                println!("{}", log_entry);

                if let Ok(text) = message.to_text() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        println!("Parsed JSON: {}", serde_json::to_string_pretty(&json)?);

                        if let Some(op) = json["op"].as_u64() {
                            println!("Opcode: {}", op);
                        }

                        if let Some(event_type) = json["t"].as_str() {
                            println!("Event type: {}", event_type)
                        }

                        if let Some(author_username) = json.pointer("/d/author/username").and_then(|v| v.as_str()) {
                            println!("Author username: {}", author_username)
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

/*
Parsed JSON: {
  "d": {
    "attachments": [],
    "author": {
      "avatar": null,
      "avatar_decoration_data": null,
      "bot": true,
      "clan": null,
      "collectibles": null,
      "discriminator": "5500",
      "display_name_styles": null,
      "global_name": null,
      "id": "1285564805838536705",
      "primary_guild": null,
      "public_flags": 0,
      "username": "Pyro Asa CrossChat"
    },
    "channel_id": "1367846456190570497",
    "channel_type": 0,
    "components": [],
    "content": "[Isl 1] WildcardsTrash [Purina]: no thats wrong",
    "edited_timestamp": null,
    "embeds": [],
    "flags": 0,
    "guild_id": "933846351245090856",
    "id": "1439319455845978144",
    "member": {
      "avatar": null,
      "banner": null,
      "communication_disabled_until": null,
      "deaf": false,
      "flags": 0,
      "joined_at": "2025-05-23T14:44:49.955000+00:00",
      "mute": false,
      "nick": "Xavii asa crosschat",
      "pending": false,
      "premium_since": null,
      "roles": [
        "987549124490559528",
        "1375484623479374046"
      ]
    },
    "mention_everyone": false,
    "mention_roles": [],
    "mentions": [],
    "pinned": false,
    "timestamp": "2025-11-15T18:21:39.536000+00:00",
    "tts": false,
    "type": 0
  },
  "op": 0,
  "s": 8,
  "t": "MESSAGE_CREATE"
}
Opcode: 0
Event type: MESSAGE_CREATE
*/
