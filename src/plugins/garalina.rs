use crate::AppState;
use bevy::prelude::*;
use bevy::window::WindowResized;

pub struct GaralinaPlugin;
impl Plugin for GaralinaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Garalina), setup_garalina)
            .add_systems(
                Update,
                (update_garalina, check_for_window_resize).run_if(in_state(AppState::Garalina)),
            )
            .add_systems(OnExit(AppState::Garalina), cleanup_garalina);
    }
}

// Constants
const DEFAULT_WIDTH: f32 = 1280.0;
const DEFAULT_HEIGHT: f32 = 720.0;
const BACKGROUND_COLOR: Color = Color::srgb_u8(153, 153, 153);
/// garalina.ogg lasts 9 seconds.
const LOGO_DURATION: f32 = 9.0;
const FADE_OUT_DURATION: f32 = 1.0;
// Components
/// A marker used to identify the mesh that fades to white at the end of GameState::Garalina.
#[derive(Component)]
struct FadingMesh;
// Resources
#[derive(Resource)]
struct MusicTimer(Timer);
#[derive(Resource)]
struct FadeOutTimer(Timer);
// Systems
fn setup_garalina(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((StateScoped(AppState::Garalina), Camera2d));
    commands.spawn((
        StateScoped(AppState::Garalina),
        AudioPlayer::new(asset_server.load("garalina/garalina.ogg")),
    ));
    commands.spawn((
        StateScoped(AppState::Garalina),
        Mesh2d(meshes.add(Rectangle::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))),
        MeshMaterial2d(materials.add(BACKGROUND_COLOR)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    commands.spawn((
        StateScoped(AppState::Garalina),
        Sprite {
            image: asset_server.load("garalina/logo_1.png"),
            custom_size: Some(Vec2::new(DEFAULT_WIDTH, DEFAULT_HEIGHT)),
            image_mode: SpriteImageMode::Scale(ScalingMode::FitCenter).into(),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
    commands.spawn((
        StateScoped(AppState::Garalina),
        FadingMesh,
        Mesh2d(meshes.add(Rectangle::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))),
        MeshMaterial2d(materials.add(Color::srgba(1.0, 1.0, 1.0, 0.0))),
        Transform::from_xyz(0.0, 0.0, 2.0),
    ));
    commands.insert_resource(MusicTimer(Timer::from_seconds(
        LOGO_DURATION,
        TimerMode::Once,
    )));
    commands.insert_resource(FadeOutTimer(Timer::from_seconds(
        FADE_OUT_DURATION,
        TimerMode::Once,
    )))
}
fn update_garalina(
    time: Res<Time>,
    mut music_timer: ResMut<MusicTimer>,
    mut fade_out_timer: ResMut<FadeOutTimer>,
    fading_mesh: Single<&MeshMaterial2d<ColorMaterial>, With<FadingMesh>>,
    mut assets: ResMut<Assets<ColorMaterial>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if music_timer.0.tick(time.delta()).finished() {
        fade_out_timer.0.tick(time.delta());
        if let Some(material) = assets.get_mut(fading_mesh.id()) {
            *material = Color::srgba(
                1.0,
                1.0,
                1.0,
                fade_out_timer.0.elapsed_secs() / FADE_OUT_DURATION,
            )
            .into();
        }
        if fade_out_timer.0.just_finished() {
            next_state.set(AppState::MainMenu);
        }
    }
}
fn check_for_window_resize(
    mut resize_reader: EventReader<WindowResized>,
    mut logo_sprite: Single<&mut Sprite>,
    mut assets: ResMut<Assets<Mesh>>,
    meshes: Query<&Mesh2d>,
) {
    // If the window was resized, resize the logo and background mesh
    if let Some(resize) = resize_reader.read().last() {
        info!("Resizing to {:?}, {:?}", resize.width, resize.height);
        logo_sprite.custom_size = Some(Vec2::new(resize.width, resize.height));
        for mesh in meshes.iter() {
            if let Some(mesh) = assets.get_mut(mesh.id()) {
                *mesh = Rectangle::new(resize.width, resize.height).into();
            }
        }
    }
}
fn cleanup_garalina(mut commands: Commands) {
    commands.remove_resource::<MusicTimer>();
    commands.remove_resource::<FadeOutTimer>();
}
