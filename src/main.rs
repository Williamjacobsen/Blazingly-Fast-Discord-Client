use dotenv::dotenv;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Deserializer};
use slint::{ModelRc, SharedString, Weak};
use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{env, time::Duration};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let authorization_token = env::var("DISCORD_TOKEN").expect("PANIC: No DISCORD_TOKEN set.");

    let http_client = Arc::new(reqwest::Client::new());

    let ui = AppWindow::new()?;
    let weak_ui = ui.as_weak();

    let app_state = Arc::new(RwLock::new(AppState::new(weak_ui)));

    let state_clone = app_state.clone();
    let auth_token_clone = authorization_token.clone();
    tokio::spawn(async move {
        let _ = handle_websocket_connection(state_clone, auth_token_clone).await;
    });

    let auth_token_clone = authorization_token.clone();
    wire_ui_events(
        &ui,
        app_state.clone(),
        http_client.clone(),
        auth_token_clone,
    );

    ui.run()?;

    Ok(())
}

fn wire_ui_events(
    ui: &AppWindow,
    app_state: Arc<RwLock<AppState>>,
    http_client: Arc<reqwest::Client>,
    authorization_token: String,
) {
    let state_clone = app_state.clone();
    let http_client_clone = http_client.clone();
    let authorization_token_clone = authorization_token.clone();

    ui.on_private_channel_clicked(move |channel_index| {
        let state = state_clone.clone();
        let http_client = http_client_clone.clone();
        let authorization_token = authorization_token_clone.clone();

        tokio::spawn(async move {
            let state = state.read().await;

            if let Some(channel) = state.private_channels.get(channel_index as usize) {
                println!("Clicked channel id: {}", channel.id);

                let response = http_client
                    .get(format!(
                        "https://discord.com/api/v9/channels/{}/messages?limit=11",
                        channel.id
                    ))
                    .header("Authorization", authorization_token)
                    .send()
                    .await
                    .unwrap();

                println!("Status: {}", response.status());
                let body = response.text().await.unwrap();
                println!("{}", body);
            }
        });
    });
}

async fn parse_initial_data(app_state: Arc<RwLock<AppState>>, payload: serde_json::Value) {
    let mut state = app_state.write().await;

    /* Get client profile information:
     *      - id
     *      - computes display_name
     *      - username
     *      - global_name
     *      - avatar_hash
     */
    println!("Getting client profile information...");

    if let Some(user) = payload.pointer("/d/user") {
        let user: Option<User> = serde_json::from_value(user.clone()).unwrap_or_default();
        println!("Logged in user info: {:?}", user);

        if let Some(mut u) = user.clone() {
            u.compute_display_name();
            state.set_client_user(Some(u));
        } else {
            state.set_client_user(user);
        }
    }

    /* Get private channels:
     *      - id
     *      - last_message_id
     *      - recipient ids
     * Also updates HashMap<id, User>
     */
    println!("Getting private channel information...");

    if let Some(private_channels) = payload.pointer("/d/private_channels") {
        if let Some(channels_array) = private_channels.as_array() {
            for channel in channels_array {
                // Store recipients in AppState as Users.
                if let Some(recipients) = channel.get("recipients").and_then(|r| r.as_array()) {
                    for recipient in recipients {
                        let mut user: User = serde_json::from_value(recipient.clone()).unwrap();
                        println!("User: {:?}", user);

                        user.compute_display_name();

                        state.users.insert(user.id, user);
                    }
                }

                // Store only the ids of recipients, and other meta data.
                let mut channel: PrivateChannel = serde_json::from_value(channel.clone()).unwrap();
                channel.compute_display_name(&state.users);
                println!("Private channel: {:?}", channel);
                state.private_channels.push(channel);
            }

            let channels = state.private_channels.clone();
            state.set_private_channels(channels);
        }
    }

    /* Get guilds:
     *      - TODO
     */
    println!("Getting guilds...");
}

async fn handle_websocket_connection(
    app_state: Arc<RwLock<AppState>>,
    authorization_token: String,
) -> Result<(), Box<dyn Error>> {
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
                                        /* "READY" event type:
                                         * Handles initial data requests by `Identify`.
                                         * This loads the following data into state:
                                         *      - Client profile information
                                         *      - Private channels
                                         *      - Guilds
                                         */
                                        "READY" => {
                                            println!(
                                                "Received READY event, parsing initial data..."
                                            );
                                            let _ = parse_initial_data(
                                                app_state.clone(),
                                                payload.clone(),
                                            )
                                            .await;
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

type Id = u64;

/*
 * AppState could be replaced by storing state in slint (avoids duplicate state).
 */
#[derive(Clone, Default)]
struct AppState {
    weak_ui: slint::Weak<AppWindow>,
    pub client_user: Option<User>,
    pub users: HashMap<Id, User>,
    pub private_channels: Vec<PrivateChannel>,
}

impl AppState {
    fn new(weak_ui: slint::Weak<AppWindow>) -> Self {
        Self {
            weak_ui,
            ..Default::default()
        }
    }

    pub fn set_client_user(&mut self, user: Option<User>) {
        self.client_user = user.clone();

        let display_name = user
            .map(|u| u.display_name)
            .unwrap_or("Error: No Username Found.".to_string())
            .to_string();

        let weak_ui_clone = self.weak_ui.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak_ui_clone.upgrade() {
                ui.set_visible_name(SharedString::from(display_name));
            }
        })
        .unwrap();
    }

    pub fn set_private_channels(&mut self, private_channels: Vec<PrivateChannel>) {
        self.private_channels = private_channels.clone();

        let names: Vec<SharedString> = self
            .private_channels
            .iter()
            .filter_map(|channel| channel.display_name.clone())
            .map(SharedString::from)
            .collect();

        let weak_ui_clone = self.weak_ui.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak_ui_clone.upgrade() {
                ui.set_private_channel_names(ModelRc::from(names.as_slice()));
            }
        })
        .unwrap();
    }
}

#[derive(Debug, Clone, Deserialize)]
struct User {
    #[serde(deserialize_with = "string_to_u64")]
    pub id: u64,

    #[serde(default)]
    pub display_name: String,

    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
}

impl User {
    pub fn compute_display_name(&mut self) {
        self.display_name = self
            .global_name
            .clone()
            .unwrap_or_else(|| self.username.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PrivateChannel {
    #[serde(deserialize_with = "string_to_u64")]
    pub id: u64,

    #[serde(rename = "name")]
    pub display_name: Option<String>,

    #[serde(deserialize_with = "optional_string_to_u64")]
    pub last_message_id: Option<u64>,

    #[serde(rename = "recipients", deserialize_with = "deserialize_recipient_ids")]
    pub recipient_ids: Vec<u64>,
}

impl PrivateChannel {
    pub fn compute_display_name(&mut self, users: &HashMap<Id, User>) {
        if self.display_name.is_none() {
            let names: Vec<String> = self
                .recipient_ids
                .iter()
                .filter_map(|id| users.get(id))
                .map(|u| u.display_name.clone())
                .collect();

            self.display_name = Some(names.join(", "));
        }
    }
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
