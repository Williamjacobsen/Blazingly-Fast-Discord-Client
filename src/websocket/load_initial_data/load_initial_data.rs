use serde_json::Value;

use crate::websocket::load_initial_data::get_client_username::get_client_username;
use crate::websocket::load_initial_data::get_private_channels::get_private_channels;

/// load_initial_data loads data received from sending the initial intent message (opcode 2).
pub fn load_initial_data(json: &Value) {
    get_client_username(json);
    get_private_channels(json);
}