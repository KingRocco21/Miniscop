use crate::states::AppState;
use bevy::prelude::*;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Overworld), setup_overworld);
    }
}

fn setup_overworld(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Main 3D camera
    commands.spawn((
        StateScoped(AppState::Overworld),
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        StateScoped(AppState::Overworld),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
    ));
    commands.spawn((
        StateScoped(AppState::Overworld),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("mainmenu/3d/Title.glb"))),
    ));
    // Animate with these functions:
    // sin(2*pi*x)*cos(pi/6*x)
    // sin(2*pi*x)*sin(pi/6*x)
}
