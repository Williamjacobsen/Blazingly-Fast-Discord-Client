use std::collections::HashSet;

use futures_util::future::join_all;
use serde_json::Value;
use tokio::spawn;

use crate::state::{AppState, ChannelType, PrivateChannel, UpdateSender, User};

pub fn get_private_channels(json: &Value) -> Vec<PrivateChannel> {
    let mut channels = Vec::new();

    if let Some(private_channels) = json
        .pointer("/d/private_channels")
        .and_then(|v| v.as_array())
    {
        for private_channel in private_channels {
            if let Some(recipients) = private_channel.get("recipients").and_then(|r| r.as_array()) {
                let _type = private_channel
                    .get("type")
                    .and_then(|v| v.as_u64())
                    .unwrap_or_default();

                let channel_type = match _type {
                    3 => ChannelType::Group,
                    1 => ChannelType::Private,
                    _ => ChannelType::Private,
                };

                let id = private_channel
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                let name = private_channel
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                let user_recipients: Vec<User> = recipients
                    .iter()
                    .filter_map(|recipient| {
                        let id = recipient
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();

                        let username = recipient
                            .get("username")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let global_name = recipient
                            .get("global_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let avatar_hash = recipient
                            .get("avatar")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();

                        if !username.is_empty() || !global_name.is_empty() {
                            Some(User {
                                id,
                                username,
                                global_name,
                                avatar_hash,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                let sort_id = private_channel
                    .get("last_message_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(
                        private_channel
                            .get("id")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0),
                    );

                let icon = private_channel
                    .get("icon")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                if !user_recipients.is_empty() {
                    channels.push(PrivateChannel {
                        id,
                        channel_type,
                        name,
                        recipients: user_recipients,
                        sort_id,
                        icon_hash: icon,
                    });
                }
            }
        }
    }

    return channels;
}

pub fn load_private_channel_avatars(app_state: AppState, update_sender: UpdateSender) {
    spawn(async move {
        // Get recipients avatars
        let recipients: Vec<User> = {
            let guard = app_state.read().await;

            let mut seen = HashSet::<String>::new();
            let mut uniques = Vec::<User>::new();

            for user in guard
                .private_channels
                .iter()
                .flat_map(|channel| channel.recipients.iter())
            {
                if seen.insert(user.id.clone()) {
                    uniques.push(user.clone());
                }
            }

            uniques
        };

        let mut futures = Vec::new();

        for user in recipients.into_iter() {
            let update_sender = update_sender.clone();
            futures.push(async move {
                let _ = user.get_avatar().await;
                let _ = update_sender.send(());
            });
        }

        let _ = join_all(futures).await;

        // Get channels icons
        let channel_list: Vec<PrivateChannel> = {
            let guard = app_state.read().await;
            guard.private_channels.clone()
        };

        let mut channel_futures = Vec::new();

        for channel in channel_list.into_iter() {
            let update_sender = update_sender.clone();

            channel_futures.push(async move {
                let _ = channel.get_icon().await;
                let _ = update_sender.send(());
            });
        }

        let _ = join_all(channel_futures).await;
    });
}
