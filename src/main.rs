use std::env;
use std::error::Error;

mod api;
mod state;
mod ui;
mod utils;
mod websocket;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let app_state = state::create_app_state();

    api::initialize()?;

    let app_state_for_messages = app_state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = api::get_recent_messages(app_state_for_messages, "1361299379020890212", None).await {
                eprintln!("Failed to get recent messages: {}", e);
            }
        });
    });

    let (update_sender, update_receiver) = state::create_update_channel();

    let app_state_clone = app_state.clone();
    std::thread::spawn({
        let update_sender = update_sender.clone();
        move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = websocket::websocket::connect(app_state_clone, update_sender).await
                {
                    eprintln!("WebSocket error: {}", e);
                }
            });
        }
    });

    ui::run_app(app_state, update_receiver)?;

    Ok(())
}
