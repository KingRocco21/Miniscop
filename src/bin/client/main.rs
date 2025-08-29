use crate::plugins::garalina::GaralinaPlugin;
use crate::plugins::mainmenu::MainMenuPlugin;
use crate::plugins::overworld::OverworldPlugin;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;
use bevy::text::{FontSmoothing, LineHeight};
use bevy::window::{CursorOptions, PresentMode, WindowResolution};
use bevy_skein::SkeinPlugin;
use bevy_sprite3d::Sprite3dPlugin;
use std::time::Duration;

mod plugins;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
#[states(scoped_entities)]
pub enum AppState {
    #[default]
    Garalina,
    MainMenu,
    Overworld,
}

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
                        resolution: WindowResolution::new(960.0, 720.0), // Petscop: 960x720. Actual PS1: 720x540.
                        resizable: false,
                        title: String::from("Miniscop: Investigate Together!"),
                        name: Some(String::from("Miniscop")),
                        prevent_default_event_handling: true,
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            SkeinPlugin::default(),
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
        .add_plugins((GaralinaPlugin, MainMenuPlugin, OverworldPlugin))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Resource, Deref, DerefMut)]
pub struct PetscopFont(TextFont);
// Systems
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut fps_overlay_config: ResMut<FpsOverlayConfig>,
) {
    let petscop_font_handle = asset_server.load::<Font>("global/fonts/PetscopWide.ttf");
    let petscop_font = TextFont {
        font: petscop_font_handle,
        font_size: 30.0,
        line_height: LineHeight::RelativeToFont(1.0),
        font_smoothing: FontSmoothing::None,
    };
    fps_overlay_config.text_config = petscop_font.clone();
    commands.insert_resource(PetscopFont(petscop_font));
    // Possible fix for overlay bugs: get entity and insert renderlayer or UITargetCamera
}
