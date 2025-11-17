use serde_json::json;
use std::error::Error;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;

pub async fn send_identity(
    authorization_token: &str,
    transmitter: UnboundedSender<Message>,
) -> Result<(), Box<dyn Error>> {
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

    Ok(())
}
