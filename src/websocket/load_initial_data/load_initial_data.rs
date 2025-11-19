use serde_json::Value;

use crate::state::AppState;
use crate::websocket::load_initial_data::get_client_username::get_client_username;
use crate::websocket::load_initial_data::get_guilds::get_guilds;
use crate::websocket::load_initial_data::get_private_channels::get_private_channels;

/// load_initial_data loads data received from sending the initial intent message (opcode 2).
pub async fn load_initial_data(json: &Value, app_state: AppState) {
    let client_user = get_client_username(json);
    let private_channels = get_private_channels(json);
    let guilds = get_guilds(json);

    let mut app_data = app_state.write().await;

    if let Some(user) = client_user {
        app_data.current_user = Some(user);
    }

    app_data.private_channels = private_channels;
    app_data.guilds = guilds;
}
