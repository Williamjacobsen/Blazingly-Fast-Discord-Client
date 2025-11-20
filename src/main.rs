use std::env;
use std::error::Error;

mod api;
mod state;
mod ui;
mod utils;
mod websocket;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    api::initialize()?;

    api::fetch_profile_information("545218808806375439")?;

    let app_state = state::create_app_state();
    let (update_sender, update_receiver) = state::create_update_channel();

    let app_state_clone = app_state.clone();
    std::thread::spawn({
        let update_sender = update_sender.clone();
        move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = websocket::websocket::connect(app_state_clone, update_sender).await {
                    eprintln!("WebSocket error: {}", e);
                }
            });
        }
    });

    ui::run_app(app_state, update_receiver)?;

    Ok(())
}