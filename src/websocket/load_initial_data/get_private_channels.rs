use serde_json::Value;

pub fn get_private_channels(json: &Value) {
    if let Some(private_channels) = json
        .pointer("/d/private_channels")
        .and_then(|v| v.as_array())
    {
        for private_channel in private_channels {
            if let Some(recipients) = private_channel.get("recipients").and_then(|r| r.as_array()) {
                if recipients.len() >= 2 {
                    let group_name = private_channel
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("<no group name>");
                    println!("Group name: {}", group_name)
                }

                for recipient in recipients {
                    let name = recipient
                        .get("global_name")
                        .and_then(|v| v.as_str())
                        .or_else(|| recipient.get("username").and_then(|v| v.as_str()))
                        .unwrap_or("<no name>");
                    println!("Recipient: {}", name);
                }
            } else {
                println!("No recipients or not an array: {}", private_channel);
            }
        }
    }
}
