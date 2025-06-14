use crate::states::AppState;
use bevy::prelude::*;
use bevy_sprite3d::{Sprite3dBuilder, Sprite3dParams};

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<OverworldState>()
            .add_systems(OnEnter(AppState::Overworld), setup_overworld)
            .add_systems(
                Update,
                finish_loading.run_if(in_state(OverworldState::Loading)),
            );
    }
}

// Overworld Sub-States
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::Overworld)]
#[states(scoped_entities)]
enum OverworldState {
    #[default]
    Loading,
    InGame,
}

// Components
#[derive(Resource)]
struct SpriteToBeSpawned(Handle<Image>);

// Systems
fn setup_overworld(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Main 3D camera
    commands.spawn((
        StateScoped(AppState::Overworld),
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::WHITE),
            ..default()
        },
        Transform::from_xyz(0.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        AmbientLight {
            brightness: 100.0,
            ..default()
        },
    ));
    // Spawn blender scene
    commands.spawn((
        StateScoped(AppState::Overworld),
        SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("overworld/3d/Gift_Plane.glb")),
        ),
        Transform::default(),
    ));
    // Start loading guardian sprite
    commands.insert_resource(SpriteToBeSpawned(
        asset_server.load("overworld/2d/guardian.png"),
    ));
}

fn finish_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sprite_to_be_spawned: Res<SpriteToBeSpawned>,
    mut sprite3d_params: Sprite3dParams,
    mut next_state: ResMut<NextState<OverworldState>>,
) {
    if asset_server
        .get_load_state(sprite_to_be_spawned.0.id())
        .is_some_and(|asset| asset.is_loaded())
    {
        commands.spawn((
            StateScoped(AppState::Overworld),
            Sprite3dBuilder {
                image: sprite_to_be_spawned.0.clone(),
                pixels_per_metre: 180.0,
                double_sided: false,
                unlit: true,
                ..default()
            }
            .bundle(&mut sprite3d_params),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ));
        commands.remove_resource::<SpriteToBeSpawned>();
        next_state.set(OverworldState::InGame);
    }
}
