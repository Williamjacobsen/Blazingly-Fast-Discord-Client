use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub global_name: String,
}

impl User {
    pub fn display_name(&self) -> &str {
        if !self.global_name.is_empty() {
            &self.global_name
        } else {
            &self.username
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    Private,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateChannel {
    pub channel_type: ChannelType,
    pub name: Option<String>,
    pub recipients: Vec<User>,
    /// sort_id is either a snowflake id of the last message sent, or a snowflake id of the channels creation.
    pub sort_id: u64,
}

impl PrivateChannel {
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            let recipient_names: Vec<String> = self
                .recipients
                .iter()
                .map(|user| user.display_name().to_string())
                .collect();

            if recipient_names.is_empty() {
                "<no recipients>".to_string()
            } else {
                recipient_names.join(", ")
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub name: String,
}

#[derive(Debug, Default)]
pub struct AppData {
    pub current_user: Option<User>,
    pub private_channels: Vec<PrivateChannel>,
    pub guilds: Vec<Guild>,
}

pub type AppState = Arc<RwLock<AppData>>;

pub fn create_app_state() -> AppState {
    Arc::new(RwLock::new(AppData::default()))
}

pub type UpdateSender = mpsc::UnboundedSender<()>;
pub type UpdateReceiver = mpsc::UnboundedReceiver<()>;

/// Used for sending () as a UI update signal.
pub fn create_update_channel() -> (UpdateSender, UpdateReceiver) {
    mpsc::unbounded_channel()
}
