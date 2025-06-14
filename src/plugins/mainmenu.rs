use crate::states::AppState;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::RenderLayers;
use bevy::text::FontSmoothing;
use bevy::ui::PositionType;

pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), setup_main_menu);
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
    // Main 3D Camera
    commands.spawn((
        StateScoped(AppState::MainMenu),
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        RenderLayers::layer(0),
    ));
    // Logo camera rendered as an image
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
            clear_color: ClearColorConfig::Custom(Color::WHITE), // Change this to transparency if you get smearing
            order: 0,
            ..default()
        },
        RenderLayers::layer(1),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Logo
    commands.spawn((
        StateScoped(AppState::MainMenu),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("mainmenu/3d/Title.glb"))),
        Transform::default(),
        Rotatable,
        RenderLayers::layer(1),
    ));
    // Logo lighting
    commands.spawn((
        StateScoped(AppState::MainMenu),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        RenderLayers::layer(1),
    ));
    // Font
    let petscop_font = asset_server.load::<Font>("global/fonts/PetscopWide.ttf");
    // UI
    commands.spawn((
        StateScoped(AppState::MainMenu),
        RenderLayers::layer(0),
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
                            ..default()
                        }
                    ),
                    (
                        ImageNode::new(camera_as_image_handle),
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Percent(40.0),
                            ..default()
                        },
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
