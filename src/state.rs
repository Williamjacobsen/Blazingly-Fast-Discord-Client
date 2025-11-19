use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub global_name: String,
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
}

#[derive(Debug, Default)]
pub struct AppData {
    pub current_user: Option<User>,
    pub private_channels: Vec<PrivateChannel>,
}

pub type AppState = Arc<RwLock<AppData>>;

pub fn create_app_state() -> AppState {
    Arc::new(RwLock::new(AppData::default()))
}
