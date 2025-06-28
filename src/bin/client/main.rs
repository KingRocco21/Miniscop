use crate::networking::stop_client_runtime_on_window_close;
use crate::networking::MultiplayerState;
use crate::plugins::garalina::GaralinaPlugin;
use crate::plugins::mainmenu::MainMenuPlugin;
use crate::plugins::overworld::OverworldPlugin;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;
use bevy::text::FontSmoothing;
use bevy::window::{CursorOptions, PresentMode};
use bevy_obj::ObjPlugin;
use bevy_sprite3d::Sprite3dPlugin;
use std::time::Duration;

mod networking;
mod plugins;

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
            Sprite3dPlugin,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_color: Color::BLACK,
                    refresh_interval: Duration::from_secs(1),
                    ..default()
                },
            },
        ))
        .insert_state(AppState::Overworld)
        .insert_state(MultiplayerState::Offline)
        .add_plugins((GaralinaPlugin, MainMenuPlugin, OverworldPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            stop_client_runtime_on_window_close.run_if(in_state(MultiplayerState::Online)),
        )
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
#[states(scoped_entities)]
pub enum AppState {
    #[default]
    Garalina,
    MainMenu,
    Overworld,
}

// Systems
fn setup(mut fps_overlay_config: ResMut<FpsOverlayConfig>, asset_server: Res<AssetServer>) {
    fps_overlay_config.text_config = TextFont {
        font: asset_server.load::<Font>("global/fonts/PetscopWide.ttf"),
        font_size: 30.0,
        font_smoothing: FontSmoothing::None,
        ..default()
    }
    // Possible fix for overlay bugs: get entity and insert renderlayer or UITargetCamera
}
