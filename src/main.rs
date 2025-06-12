use crate::plugins::garalina::GaralinaPlugin;
use crate::plugins::mainmenu::MainMenuPlugin;
use crate::states::AppState;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;
use bevy::text::FontSmoothing;
use bevy::window::{CursorOptions, PresentMode};
use bevy_obj::ObjPlugin;

mod plugins;
mod states;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        cursor_options: CursorOptions {
                            visible: false,
                            ..default()
                        },
                        present_mode: PresentMode::AutoVsync,
                        // mode: WindowMode::Fullscreen(
                        //     MonitorSelection::Primary,
                        //     VideoModeSelection::Current,
                        // ),
                        title: "Miniscop: Investigate Together!".to_string(),
                        name: Some("Miniscop".to_string()),
                        prevent_default_event_handling: false, // Setting it to false means you should not bind inputs to F5, F12, Ctrl+R, and Tab
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            ObjPlugin,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_color: Color::WHITE,
                    ..default()
                },
            },
        ))
        .insert_state(AppState::MainMenu)
        .add_plugins((GaralinaPlugin, MainMenuPlugin))
        .add_systems(Startup, setup)
        .run();
}

// Systems
fn setup(mut fps_overlay_config: ResMut<FpsOverlayConfig>, asset_server: Res<AssetServer>) {
    fps_overlay_config.text_config = TextFont {
        font: asset_server.load::<Font>("global/fonts/PetscopWide.ttf"),
        font_size: 30.0,
        font_smoothing: FontSmoothing::None,
        ..default()
    }
}
