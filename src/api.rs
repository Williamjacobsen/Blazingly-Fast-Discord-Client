use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::{env, error::Error};

use dotenv::dotenv;

use crate::state::{AppState, Message, HTTP_CLIENT};

pub fn initialize() -> Result<(), Box<dyn Error>> {
    dotenv()?;
    Ok(())
}

pub fn get_recent_messages(app_state: AppState) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = _get_recent_messages(app_state, "1361299379020890212", None).await {
                eprintln!("Failed to get recent messages: {}", e);
            }
        });
    });
}

async fn _get_recent_messages(
    app_state: AppState,
    channel_id: &str,
    limit: Option<u8>,
) -> Result<(), Box<dyn Error>> {
    let limit = limit.unwrap_or(50).min(100);

    let authorization_token = env::var("DISCORD_TOKEN")?;
    let url = format!(
        "https://discord.com/api/v9/channels/{}/messages?limit={}",
        channel_id, limit
    );

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&authorization_token)?);

    let response = HTTP_CLIENT.get(&url).headers(headers).send().await?;

    if !response.status().is_success() {
        eprintln!(
            "Failed to fetch messages for channel {}: {}",
            channel_id,
            response.status()
        );
        return Ok(());
    }

    let json: Value = response.json().await?;

    let messages: Vec<Message> = if let Some(arr) = json.as_array() {
        arr.iter()
            .filter_map(|msg| parse_message(msg).ok())
            .collect()
    } else {
        vec![]
    };

    let mut state = app_state.write().await;
    if let Some(channel) = state
        .private_channels
        .iter_mut()
        .find(|ch| ch.id == channel_id)
    {
        channel.messages = Some(messages);
    } else {
        eprintln!("Channel {} not found in app_state", channel_id);
    }

    Ok(())
}

fn parse_message(value: &Value) -> Result<Message, Box<dyn Error>> {
    let id = value["id"].as_str().unwrap_or_default().to_string();
    let _type = value["type"].as_u64().unwrap_or(0) as u8;
    let content = value["content"].as_str().unwrap_or_default().to_string();

    let mentions = value["mentions"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|u| u["id"].as_str().map(String::from))
            .collect()
    });

    let mention_everyone = value["mention_everyone"].as_bool().unwrap_or(false);
    let timestamp = value["timestamp"].as_str().unwrap_or_default().to_string();

    let edited_timestamp = value["edited_timestamp"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from);

    let author_id = value["author"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    Ok(Message {
        id,
        _type,
        content,
        mentions,
        mention_everyone,
        timestamp,
        edited_timestamp,
        author_id,
    })
}
