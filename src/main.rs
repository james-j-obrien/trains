use bevy::{prelude::*, render::camera::RenderTarget};
use bevy_egui::EguiPlugin;
use bevy_mod_picking::{
    DebugCursorPickingPlugin, DebugEventsPickingPlugin, DefaultPickingPlugins, PickingCameraBundle,
};
use bevy_prototype_lyon::{prelude::*, shapes};

mod camera;
use camera::*;

mod constants;
use constants::*;

mod graph;
use graph::*;

mod placement;
use placement::*;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "trains".to_string(),
            height: WINDOW_HEIGHT,
            width: WINDOW_WIDTH,
            resizable: false,
            ..default()
        })
        .insert_resource(ClearColor(Color::rgb_u8(42, 42, 42)))
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(EguiPlugin)
        // .add_plugin(DebugCursorPickingPlugin) // <- Adds the green debug cursor.
        // .add_plugin(DebugEventsPickingPlugin)
        .add_plugin(ShapePlugin) // <- Adds debug event logging.
        .add_startup_system(setup)
        .add_startup_system(setup_network)
        .insert_resource(MousePos(None))
        .insert_resource(PlacementState::default())
        .add_system(camera_pan.before(mouse_to_world))
        .add_system(camera_zoom.before(mouse_to_world))
        .add_system(mouse_to_world)
        .add_system(highlight.after(mouse_to_world))
        .add_system(extract_network_to_mesh)
        .add_startup_system(setup_placement)
        .add_system(placement.after(mouse_to_world))
        .add_system(track_control)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());

    // Spawn highlight entity
    let square = shapes::RegularPolygon {
        sides: 4,
        feature: shapes::RegularPolygonFeature::SideLength(TILE_SIZE),
        ..default()
    };

    commands
        .spawn_bundle(GeometryBuilder::build_as(
            &square,
            DrawMode::Fill(FillMode::color(Color::rgba(1., 1., 1., 0.01))),
            Transform::default(),
        ))
        .insert(Highlight);
}

type TilePos = IVec2;

fn world_pos_to_tile(pos: Vec2) -> TilePos {
    (pos / TILE_SIZE - 0.5).round().as_ivec2()
}

fn tile_to_world_pos(tile: TilePos) -> Vec2 {
    tile.as_vec2() * TILE_SIZE
}

fn tile_to_center_pos(tile: TilePos) -> Vec2 {
    tile_to_world_pos(tile) + Vec2::splat(TILE_SIZE / 2.)
}

#[derive(Debug)]
pub struct MousePos(Option<Vec2>);

fn mouse_to_world(
    mut mouse_pos: ResMut<MousePos>,
    wnds: Res<Windows>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let (camera, camera_transform) = q_camera.single();

    let wnd = if let RenderTarget::Window(id) = camera.target {
        wnds.get(id).unwrap()
    } else {
        wnds.get_primary().unwrap()
    };

    if let Some(screen_pos) = wnd.cursor_position() {
        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        mouse_pos.0 = Some(world_pos.truncate());
    } else {
        mouse_pos.0 = None;
    }
}

#[derive(Component)]
struct Highlight;

fn highlight(
    mouse_pos: Res<MousePos>,
    mut highlight: Query<(&mut Transform, &mut Visibility), With<Highlight>>,
) {
    let (mut tf, mut vis) = highlight.single_mut();
    if let Some(mouse_pos) = mouse_pos.0 {
        let tile = world_pos_to_tile(mouse_pos);
        let pos = tile_to_world_pos(tile);

        tf.translation.x = pos.x + TILE_SIZE / 2.;
        tf.translation.y = pos.y + TILE_SIZE / 2.;
        vis.is_visible = true;
    } else {
        vis.is_visible = false;
    }
}
