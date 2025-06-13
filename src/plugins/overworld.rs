use crate::states::AppState;
use bevy::math::ops::{cos, sin};
use bevy::prelude::*;
use std::f32::consts::PI;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Overworld), setup_overworld)
            .add_systems(Update, update_title.run_if(in_state(AppState::Overworld)));
    }
}

// Components
#[derive(Component)]
struct Rotatable;

// Systems
fn setup_overworld(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Main 3D camera
    commands.spawn((
        StateScoped(AppState::Overworld),
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        Transform::default(),
        Rotatable,
    ));
}

fn update_title(mut title: Single<&mut Transform, With<Rotatable>>, mut time: Res<Time>) {
    let seconds = time.elapsed_secs();
    let theta_y = 15.0 * sin(2.0 * PI * seconds) * cos(PI / 6.0 * seconds) * PI / 180.0;
    let theta_z = 15.0 * sin(2.0 * PI * seconds) * sin(PI / 6.0 * seconds) * PI / 180.0;
    title.rotation = Quat::from_euler(EulerRot::XYZEx, 0.0, theta_y, theta_z);
}
