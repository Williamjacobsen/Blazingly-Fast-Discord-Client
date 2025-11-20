use std::{error::Error, sync::Arc};

use futures_util::{stream::SplitStream, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::{
    state::{AppState, UpdateSender},
    websocket::{
        load_initial_data::load_initial_data::load_initial_data, sequence_tracker::SequenceTracker,
    },
};

/// Handles incomming messages.
///
/// opcodes:
/// - 0 Dispatch (Receive): An event was dispatched.
/// - 0 and s == 0, then it contains almost all data requested intent (2).
/// - 1 Heartbeat (Send/Receive): Fired periodically by the client to keep the connection alive.
/// - 2 Identify (Send): Starts a new session during the initial handshake.
/// - 3 Presence Update (Send): Update the client’s presence.
/// - 4 Voice State Update (Send): Used to join/leave or move between voice channels.
/// - 6 Resume (Send): Resume a previous session that was disconnected.
/// - 7 Reconnect (Receive): You should attempt to reconnect and resume immediately.
/// - 8 Request Guild Members (Send): Request information about offline guild members in a large guild.
/// - 9 Invalid Session (Receive): The session has been invalidated. You should reconnect and identify/resume accordingly.
/// - 10 Hello (Receive): Sent immediately after connecting, contains the heartbeat_interval to use.
/// - 11 Heartbeat ACK (Receive): Sent in response to receiving a heartbeat to acknowledge that it has been received.
/// - 31 Request Soundboard Sounds (Send): Request information about soundboard sounds in a set of guilds.
pub async fn handle_incomming_messages(
    read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    sequence_tracker: Arc<SequenceTracker>,
    app_state: AppState,
    update_sender: UpdateSender
) -> Result<(), Box<dyn Error>> {
    while let Some(message) = read.next().await {
        match message {
            Ok(message) => {
                if let Ok(text) = message.to_text() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        println!("Parsed JSON: {}", serde_json::to_string_pretty(&json)?);

                        if let Some(s) = json["s"].as_u64() {
                            if s > sequence_tracker.get() {
                                sequence_tracker.update(s);
                            } else {
                                eprintln!("Wrong seq_num: {}", s);
                            }
                        }

                        if let (Some(op), Some(s), Some(t)) =
                            (json["op"].as_u64(), json["s"].as_u64(), json["t"].as_str())
                        {
                            println!("Opcode: {}", op);
                            println!("Sequence number: {}", s);
                            println!("Event type: {}", t);

                            if s == 1 && op == 0 {
                                load_initial_data(&json, app_state.clone()).await;

                                let _ = update_sender.send(());

                                let app_data = app_state.read().await;
                                println!("{:?}", *app_data);
                            } else if op == 0 {
                                // Should check event type "t".
                                //if let Some(author_username) =
                                //    json.pointer("/d/author/username").and_then(|v| v.as_str())
                                //{
                                //    println!("Author username: {}", author_username)
                                //}
                            } else {
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

    Ok(())
}
