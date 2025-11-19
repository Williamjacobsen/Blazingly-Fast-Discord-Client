use serde_json::Value;

use crate::state::User;

pub fn get_client_username(json: &Value) -> Option<User> {
    if let Some(user_obj) = json.pointer("/d/user") {
        let username = user_obj
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let global_name = user_obj
            .get("global_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        println!("Username: {}, Global Name: {}", username, global_name);

        Some(User {
            username,
            global_name,
        })
    } else {
        None
    }
}
