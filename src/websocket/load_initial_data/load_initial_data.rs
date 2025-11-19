use serde_json::Value;

use crate::state::AppState;
use crate::websocket::load_initial_data::get_client_username::get_client_username;
use crate::websocket::load_initial_data::get_private_channels::get_private_channels;

/// load_initial_data loads data received from sending the initial intent message (opcode 2).
pub async fn load_initial_data(json: &Value, app_state: AppState) {
    let mut app_data = app_state.write().await;

    if let Some(user) = get_client_username(json) {
        app_data.current_user = Some(user);
    }

    let private_channels = get_private_channels(json);
    app_data.private_channels = private_channels;
}
