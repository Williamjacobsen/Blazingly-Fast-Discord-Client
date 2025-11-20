use serde_json::Value;

use crate::state::{ChannelType, PrivateChannel, User};

pub fn get_private_channels(json: &Value) -> Vec<PrivateChannel> {
    let mut channels = Vec::new();

    if let Some(private_channels) = json
        .pointer("/d/private_channels")
        .and_then(|v| v.as_array())
    {
        for private_channel in private_channels {
            if let Some(recipients) = private_channel.get("recipients").and_then(|r| r.as_array()) {
                let channel_type = if recipients.len() >= 2 {
                    ChannelType::Group
                } else {
                    ChannelType::Private
                };

                let name = if recipients.len() >= 2 {
                    private_channel
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                };

                let user_recipients: Vec<User> = recipients
                    .iter()
                    .filter_map(|recipient| {
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

                        if !username.is_empty() || !global_name.is_empty() {
                            Some(User {
                                username,
                                global_name,
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

                if !user_recipients.is_empty() {
                    channels.push(PrivateChannel {
                        channel_type,
                        name,
                        recipients: user_recipients,
                        sort_id,
                    });
                }
            }
        }
    }

    return channels;
}
