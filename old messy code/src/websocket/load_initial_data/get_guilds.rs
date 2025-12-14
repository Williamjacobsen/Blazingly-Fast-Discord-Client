use serde_json::Value;

use crate::state::Guild;

pub fn get_guilds(json: &Value) -> Vec<Guild> {
    let guilds = match json.pointer("/d/guilds").and_then(Value::as_array) {
        Some(arr) if !arr.is_empty() => arr,
        _ => return Vec::new(),
    };

    let mut result = Vec::with_capacity(guilds.len());
    result.extend(guilds.iter().map(|guild| {
        let name = guild
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_default(); 
        Guild { name }
    }));
    result
}
