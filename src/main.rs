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
    let (update_sender, mut update_receiver) = state::create_update_channel();

    std::thread::spawn({
        let app_state = app_state.clone();
        move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                while let Some(()) = update_receiver.recv().await {
                    println!("UI should update now!");

                    // TODO: update UI components.

                    let app_data = app_state.read().await;
                    println!("Current state: {:?}", *app_data);
                }
            })
        }
    });

    std::thread::spawn({
        let update_sender = update_sender.clone();
        move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = websocket::websocket::connect(app_state, update_sender).await {
                    eprintln!("WebSocket error: {}", e);
                }
            });
        }
    });

    ui::run_app()?;

    Ok(())
}
