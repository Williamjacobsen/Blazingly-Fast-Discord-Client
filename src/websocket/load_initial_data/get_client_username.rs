use serde_json::Value;

pub fn get_client_username(json: &Value) {
    if let Some(username) = json.pointer("/d/user/global_name").and_then(|v| v.as_str()) {
        println!("Username: {}", username);
    }
}
