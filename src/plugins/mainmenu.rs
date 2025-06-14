use crate::states::AppState;
use bevy::asset::RenderAssetUsages;
use bevy::math::ops::{cos, sin};
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::text::FontSmoothing;
use bevy::ui::PositionType;
use std::f32::consts::PI;

pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
            .add_systems(Update, update_title.run_if(in_state(AppState::MainMenu)));
    }
}

// Constants
const GIFT_ASPECT_RATIO: f32 = 88.0 / 83.0;
const LOGO_ASPECT_RATIO: f32 = 528.0 / 145.0;

#[derive(Component)]
struct Rotatable;

// Systems
fn setup_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    // Music
    // commands.spawn((
    //     StateScoped(AppState::MainMenu),
    //     AudioPlayer::new(asset_server.load("mainmenu/petscop.ogg")),
    // ));
    // Main Camera
    commands.spawn((
        StateScoped(AppState::MainMenu),
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        // https://github.com/bevyengine/bevy/issues/5183
        // RenderLayers::layer(0),
    ));
    // Title camera rendered as an image
    let mut camera_as_image = Image::new_fill(
        Extent3d {
            width: (LOGO_ASPECT_RATIO * 200.0) as u32,
            height: 200,
            ..default()
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    camera_as_image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let camera_as_image_handle = images.add(camera_as_image);
    commands.spawn((
        StateScoped(AppState::MainMenu),
        Camera3d::default(),
        Camera {
            target: RenderTarget::Image(camera_as_image_handle.clone().into()),
            clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            order: 0,
            ..default()
        },
        // https://github.com/bevyengine/bevy/issues/5183
        // RenderLayers::layer(1),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Title
    commands.spawn((
        StateScoped(AppState::MainMenu),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("mainmenu/3d/Title.glb"))),
        Transform::default(),
        Rotatable,
        // https://github.com/bevyengine/bevy/issues/5183
        // RenderLayers::layer(1),
    ));
    // Title lighting
    commands.spawn((
        StateScoped(AppState::MainMenu),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        // https://github.com/bevyengine/bevy/issues/5183
        // RenderLayers::layer(1),
    ));
    // Font
    let petscop_font = asset_server.load::<Font>("global/fonts/PetscopWide.ttf");
    // UI
    commands.spawn((
        StateScoped(AppState::MainMenu),
        // https://github.com/bevyengine/bevy/issues/5183
        // RenderLayers::layer(0),
        // Big UI with a width of twice the window width to fit multiple child UIs
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Start,
            width: Val::Percent(200.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
            ..default()
        },
        children![
            // Title Node
            (
                Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: Val::Percent(50.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Percent(10.0),
                    ..default()
                },
                children![
                    (
                        ImageNode::new(asset_server.load("mainmenu/gift.png")),
                        Transform::default(), // Todo: rotate with sine function
                        Node {
                            width: Val::Px(GIFT_ASPECT_RATIO * 300.0),
                            height: Val::Px(300.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        children![ImageNode::new(camera_as_image_handle)]
                    ),
                    (
                        Text::new("Press Z to Begin"),
                        TextColor::WHITE,
                        TextFont {
                            font: petscop_font.clone(),
                            font_size: 50.0,
                            font_smoothing: FontSmoothing::None,
                            ..default()
                        }
                    ),
                    (
                        Text::new("© 1997 Garalina"),
                        TextColor::WHITE,
                        TextFont {
                            font: petscop_font,
                            font_size: 60.0,
                            font_smoothing: FontSmoothing::None,
                            ..default()
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Percent(5.0),
                            ..default()
                        }
                    ),
                ]
            ),
            // Todo: Add rest of UI lol
        ],
    ));
}

fn update_title(mut title: Single<&mut Transform, With<Rotatable>>, time: Res<Time>) {
    let seconds = time.elapsed_secs();
    // See https://www.desmos.com/calculator/2ubcdcyfti for visualization
    // 10 degrees max in each direction
    let theta_y = sin(2.0 * PI * seconds) * cos(PI / 6.0 * seconds) * PI / 18.0;
    // 10 degrees max in each direction
    let theta_z = sin(2.0 * PI * seconds) * sin(PI / 6.0 * seconds) * PI / 18.0;
    title.rotation = Quat::from_euler(EulerRot::XYZEx, 0.0, theta_y, theta_z);
}
