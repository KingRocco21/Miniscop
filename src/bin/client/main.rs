use crate::plugins::garalina::GaralinaPlugin;
use crate::plugins::mainmenu::MainMenuPlugin;
use crate::plugins::overworld::OverworldPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::text::{FontSmoothing, LineHeight};
use bevy::window::{CursorOptions, PresentMode, WindowResolution};
use bevy_asset_loader::loading_state::LoadingState;
use bevy_asset_loader::prelude::{AssetCollection, ConfigureLoadingState, LoadingStateAppExt};
use bevy_skein::SkeinPlugin;
use bevy_sprite3d::Sprite3dPlugin;

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
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .init_state::<AppState>()
        .add_loading_state(
            LoadingState::new(AppState::Loading)
                .load_collection::<GlobalAssets>()
                .finally_init_resource::<PetscopFont>()
                .continue_to_state(AppState::Overworld),
        )
        .add_plugins((GaralinaPlugin, MainMenuPlugin, OverworldPlugin))
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
#[states(scoped_entities)]
pub enum AppState {
    #[default]
    Loading,
    Garalina,
    MainMenu,
    Overworld,
}

#[derive(AssetCollection, Resource)]
pub struct GlobalAssets {
    #[asset(path = "global/fonts/PetscopWide.ttf")]
    pub petscop_font: Handle<Font>,
}

#[derive(Resource, Deref, DerefMut)]
pub struct PetscopFont(TextFont);

// Based on https://github.com/NiklasEi/bevy_asset_loader/blob/main/bevy_asset_loader/examples/finally_init_resource.rs
impl FromWorld for PetscopFont {
    fn from_world(world: &mut World) -> Self {
        let mut system_state = SystemState::<Res<GlobalAssets>>::new(world);
        let assets = system_state.get(world);
        PetscopFont(TextFont {
            font: assets.petscop_font.clone(),
            font_size: 30.0,
            line_height: LineHeight::RelativeToFont(1.0),
            font_smoothing: FontSmoothing::None,
        })
    }
}
