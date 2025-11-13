use std::env;
use std::error::Error;

mod api;
mod ui;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    api::initialize()?;

    api::fetch_profile_information("545218808806375439")?;

    ui::run_app()?;

    Ok(())
}
