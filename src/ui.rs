use slint::{ModelRc, SharedString, VecModel};

use crate::state::{AppState, UpdateReceiver};
use std::error::Error;
slint::include_modules!();

pub fn run_app(
    app_state: AppState,
    mut update_receiver: UpdateReceiver,
) -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    {
        let guard = app_state.blocking_read();

        ui.set_visible_name(SharedString::from(
            guard
                .current_user
                .as_ref()
                .map(|user| user.display_name())
                .unwrap_or("<display_name>"),
        ));

        let private_channel_names: ModelRc<SharedString> = ModelRc::new(VecModel::from(
            guard
                .private_channels
                .iter()
                .map(|v| SharedString::from(v.display_name()))
                .collect::<Vec<SharedString>>(),
        ));
        ui.set_private_channel_names(private_channel_names);

        if let Some(user) = &guard.current_user {
            ui.set_avatar_image(user.load_avatar_image());
        }
    }

    let weak_ui = ui.as_weak();

    let app_state_clone = app_state.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            while let Some(()) = update_receiver.recv().await {
                let weak_ui = weak_ui.clone();
                let app_state = app_state_clone.clone();

                slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak_ui.upgrade() {
                        let guard = app_state.blocking_read();

                        ui.set_visible_name(SharedString::from(
                            guard
                                .current_user
                                .as_ref()
                                .map(|user| user.display_name())
                                .unwrap_or("<display_name>"),
                        ));

                        let private_channel_names: ModelRc<SharedString> =
                            ModelRc::new(VecModel::from(
                                guard
                                    .private_channels
                                    .iter()
                                    .map(|v| SharedString::from(v.display_name()))
                                    .collect::<Vec<SharedString>>(),
                            ));
                        ui.set_private_channel_names(private_channel_names);

                        if let Some(user) = &guard.current_user {
                            ui.set_avatar_image(user.load_avatar_image());
                        }
                    }
                })
                .unwrap();
            }
        });
    });

    ui.run()?;
    Ok(())
}
