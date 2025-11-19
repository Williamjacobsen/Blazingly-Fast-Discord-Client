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

    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = websocket::websocket::connect(app_state).await {
                eprintln!("WebSocket error: {}", e);
            }
        });
    });

    ui::run_app()?;

    Ok(())
}
