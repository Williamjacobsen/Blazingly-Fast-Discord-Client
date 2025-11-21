use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use slint::Image;
use std::{error::Error, path::PathBuf, sync::Arc};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{mpsc, RwLock},
};

// TODO:
// when all avatars are loaded,
// then check if a user has to different avatars saved locally,
// and delete the old one.

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent("DiscordClient") // optional but polite
        .pool_idle_timeout(std::time::Duration::from_secs(30))
        .pool_max_idle_per_host(8)
        .build()
        .expect("Failed to create global HTTP client")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub global_name: String,
    pub avatar_hash: String,
}

impl User {
    pub fn display_name(&self) -> &str {
        if !self.global_name.is_empty() {
            &self.global_name
        } else {
            &self.username
        }
    }

    fn local_avatar_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "./assets/avatars/{}_{}.png",
            self.id, self.avatar_hash
        ))
    }

    pub fn load_avatar_image(&self) -> Image {
        let path = self.local_avatar_path();

        if path.exists() {
            Image::load_from_path(&path).unwrap_or_default()
        } else {
            Image::default()
        }
    }

    pub async fn get_avatar(&self) -> Result<(), Box<dyn Error>> {
        let path = self.local_avatar_path();

        if path.exists() {
            return Ok(());
        }

        let url = format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
            self.id, self.avatar_hash
        );

        let bytes = HTTP_CLIENT.get(url).send().await?.bytes().await?;

        let folder = "./assets/avatars";
        tokio::fs::create_dir_all(folder).await?;
        let file_path = format!("{}/{}_{}.png", folder, self.id, self.avatar_hash);
        let mut file = File::create(&file_path).await?;
        file.write_all(&bytes).await?;

        println!("Hopefully saved avatar");

        Ok(())
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
