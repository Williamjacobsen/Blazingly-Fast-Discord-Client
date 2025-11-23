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

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .pool_idle_timeout(std::time::Duration::from_secs(30))
        .pool_max_idle_per_host(8)
        .build()
        .expect("Failed to create global HTTP client")
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
        if self.id.is_empty() || self.avatar_hash.is_empty() {
            return Ok(());
        }

        let path = self.local_avatar_path();

        if path.exists() {
            return Ok(());
        }

        let (extension, url) = if self.avatar_hash.starts_with("a_") {
            (
                "gif",
                format!(
                    "https://cdn.discordapp.com/avatars/{}/{}.webp?size=64",
                    self.id, self.avatar_hash
                ),
            )
        } else {
            (
                "png",
                format!(
                    "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
                    self.id, self.avatar_hash
                ),
            )
        };

        let bytes = HTTP_CLIENT.get(url).send().await?.bytes().await?;

        let folder = "./assets/avatars";
        tokio::fs::create_dir_all(folder).await?;
        let file_path = format!("{}/{}_{}.{}", folder, self.id, self.avatar_hash, extension);
        let mut file = File::create(&file_path).await?;
        file.write_all(&bytes).await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelType {
    Private,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateChannel {
    pub id: String,
    pub channel_type: ChannelType,
    pub name: String,
    pub recipients: Vec<User>,
    /// sort_id is either a snowflake id of the last message sent, or a snowflake id of the channels creation.
    pub sort_id: u64,
    pub icon_hash: String,
}

impl PrivateChannel {
    pub fn display_name(&self) -> String {
        if !self.name.is_empty() {
            self.name.clone()
        } else {
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
        }
    }

    fn local_icon_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "./assets/channel_icons/{}_{}{}.png",
            self.sort_id, self.icon_hash, ""
        ))
    }

    pub fn load_icon_image(&self) -> Image {
        let path = self.local_icon_path();

        if path.exists() {
            Image::load_from_path(&path).unwrap_or_default()
        } else {
            Image::default()
        }
    }

    pub async fn get_icon(&self) -> Result<(), Box<dyn Error>> {
        if self.icon_hash.is_empty() {
            return Ok(());
        }

        let path = self.local_icon_path();

        if path.exists() {
            return Ok(());
        }

        let (extension, url) = if self.icon_hash.starts_with("a_") {
            (
                "gif",
                format!(
                    "https://cdn.discordapp.com/channel-icons/{}/{}.webp?size=64",
                    self.id, self.icon_hash
                ),
            )
        } else {
            (
                "png",
                format!(
                    "https://cdn.discordapp.com/channel-icons/{}/{}.png?size=64",
                    self.id, self.icon_hash
                ),
            )
        };

        let bytes = HTTP_CLIENT.get(url).send().await?.bytes().await?;

        let folder = "./assets/channel_icons";
        tokio::fs::create_dir_all(folder).await?;
        let file_path = format!(
            "{}/{}_{}.{}",
            folder, self.sort_id, self.icon_hash, extension
        );
        let mut file = File::create(&file_path).await?;
        file.write_all(&bytes).await?;

        Ok(())
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
