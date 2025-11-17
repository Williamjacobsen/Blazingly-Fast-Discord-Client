use serde_json::Value;

use crate::websocket::load_initial_data::get_client_username::get_client_username;
use crate::websocket::load_initial_data::get_private_channels::get_private_channels;

pub fn load_initial_data(json: &Value) {
    get_client_username(json);
    get_private_channels(json);
}