use slint::{Image, ModelRc, SharedString, VecModel, Weak};

use crate::{
    api,
    state::{AppState, ChannelType, UpdateReceiver},
};
use std::error::Error;
slint::include_modules!();

pub fn run_app(app_state: AppState, update_receiver: UpdateReceiver) -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    update_ui(&ui, app_state.clone());

    on_private_channel_clicked(&ui, app_state.clone());

    sync_ui(app_state, &ui, update_receiver);

    ui.run()?;
    Ok(())
}

fn on_private_channel_clicked(ui: &AppWindow, app_state: AppState) {
    let weak_ui = ui.as_weak();
    ui.on_private_channel_clicked(move |index| {
        let guard = app_state.blocking_read();
        if let Some(channel) = guard.private_channels.get(index as usize) {
            println!(
                "Private channel clicked: {} (index: {})",
                channel.display_name(),
                index
            );

            let channel_id = channel.id.clone();

            drop(guard);

            get_recent_messages_async(weak_ui.clone(), app_state.clone(), channel_id);
        }
    });
}

pub fn get_recent_messages_async(
    weak_ui: Weak<AppWindow>,
    app_state: AppState,
    channel_id: String,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match api::get_recent_messages(app_state, &channel_id, Some(50)).await {
                Ok(messages) => {
                    println!("{:?}", messages);

                    let message_data: Vec<MessageData> = messages
                        .into_iter()
                        .map(|msg| MessageData {
                            author: SharedString::from(&msg.author_id),
                            content: SharedString::from(&msg.content),
                        })
                        .rev().collect();

                    if let Err(e) = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak_ui.upgrade() {
                            let messages_model = ModelRc::new(VecModel::from(message_data));
                            ui.set_messages(messages_model);
                        }
                    }) {
                        eprintln!("Failed to invoke from event loop: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("get_recent_messages failed: {}", e);
                }
            }
        });
    });
}

fn sync_ui(app_state: AppState, ui: &AppWindow, mut update_receiver: UpdateReceiver) {
    let weak_ui = ui.as_weak();
    let app_state = app_state.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            while let Some(()) = update_receiver.recv().await {
                let weak_ui = weak_ui.clone();
                let app_state = app_state.clone();

                slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak_ui.upgrade() {
                        update_ui(&ui, app_state);
                    }
                })
                .unwrap();
            }
        });
    });
}

fn update_ui(ui: &AppWindow, app_state: AppState) {
    let guard = app_state.blocking_read();

    ui.set_visible_name(SharedString::from(
        guard
            .current_user
            .as_ref()
            .map(|user| user.display_name())
            .unwrap_or("<display_name>"),
    ));

    if let Some(user) = &guard.current_user {
        ui.set_avatar_image(user.load_avatar_image());
    }

    let private_channel_names: ModelRc<SharedString> = ModelRc::new(VecModel::from(
        guard
            .private_channels
            .iter()
            .map(|v| SharedString::from(v.display_name()))
            .collect::<Vec<SharedString>>(),
    ));
    ui.set_private_channel_names(private_channel_names);

    let private_channel_avatars: ModelRc<Image> = ModelRc::new(VecModel::from(
        guard
            .private_channels
            .iter()
            .map(|channel| match channel.channel_type {
                ChannelType::Group => {
                    if !channel.icon_hash.is_empty() {
                        channel.load_icon_image()
                    } else {
                        Image::default()
                    }
                }
                ChannelType::Private => channel
                    .recipients
                    .first()
                    .map(|user| user.load_avatar_image())
                    .unwrap_or_default(),
            })
            .collect::<Vec<Image>>(),
    ));
    ui.set_private_channel_avatars(private_channel_avatars);
}
