#![allow(clippy::too_many_arguments)]

use bevy::{prelude::*, render::camera::RenderTarget};
use bevy_egui::EguiPlugin;
use bevy_egui::{egui, EguiContext};

use bevy_mod_picking::{DefaultPickingPlugins, PickingCameraBundle};
use bevy_prototype_lyon::{prelude::*, shapes};

mod camera;
use camera::*;

mod constants;
use constants::*;

mod track_graph;
use track_graph::*;

mod track_placement_tool;
use iyes_loopless::prelude::{AppLooplessStateExt, ConditionSet};
use iyes_loopless::state::{CurrentState, NextState};
use track_placement_tool::*;

mod draw;
use draw::*;

mod utils;
use utils::*;

mod track_types;
use track_types::*;

mod train_placement_tool;
use train_placement_tool::*;

pub const TITLE: &str = "Track Laying";

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum ControlState {
    None,
    PlacingTracks,
    PlacingTrains,
}

pub fn app() -> App {
    let mut app = App::new();
    app.insert_resource(WindowDescriptor {
        title: TITLE.to_string(),
        height: WINDOW_HEIGHT,
        width: WINDOW_WIDTH,
        canvas: Some("#bevy".to_string()),
        fit_canvas_to_parent: true,
        ..default()
    })
    .insert_resource(ClearColor(Color::rgb_u8(42, 42, 42)))
    .add_plugins(DefaultPlugins)
    .add_plugins(DefaultPickingPlugins)
    .add_plugin(EguiPlugin)
    // .add_plugin(DebugCursorPickingPlugin) // <- Adds the green debug cursor.
    // .add_plugin(DebugEventsPickingPlugin) // <- Adds debug event logging.
    .add_plugin(ShapePlugin)
    .add_startup_system(setup)
    .add_startup_system(setup_network)
    .add_startup_system(setup_track_placement)
    .add_loopless_state(ControlState::PlacingTracks)
    .insert_resource(MousePos(None))
    .insert_resource(PlacementState::default())
    .add_event::<TrackPlacementEvent>()
    .add_event::<NetworkRenderEvent>()
    .add_exit_system(ControlState::PlacingTracks, cleanup_track_placement)
    .add_exit_system(ControlState::PlacingTrains, cleanup_train_placement)
    .add_system(camera_pan.before(mouse_to_world))
    .add_system(camera_zoom.before(mouse_to_world))
    .add_system(mouse_to_world)
    .add_system(control_ui)
    .add_system(place_tracks)
    .add_system(extract_network_to_mesh.after(place_tracks))
    .add_system(highlight.after(mouse_to_world))
    .add_system_set(
        ConditionSet::new()
            .after(mouse_to_world)
            .run_in_state(ControlState::PlacingTracks)
            .with_system(track_placement_tool)
            .with_system(remove_tracks)
            .into(),
    )
    .add_system_set(
        ConditionSet::new()
            .after(mouse_to_world)
            .run_in_state(ControlState::PlacingTrains)
            .with_system(train_placement_tool)
            .into(),
    );
    app
}

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(Camera2dBundle::default())
        .insert_bundle(PickingCameraBundle::default());

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
        let tile = pos_to_tile_vec(mouse_pos);
        let pos = tile_vec_to_pos(tile);

        tf.translation.x = pos.x + TILE_SIZE / 2.;
        tf.translation.y = pos.y + TILE_SIZE / 2.;
        vis.is_visible = true;
    } else {
        vis.is_visible = false;
    }
}

pub fn control_ui(
    mut commands: Commands,
    state: Res<CurrentState<ControlState>>,
    mut ctx: ResMut<EguiContext>,
    mut params: ResMut<TrackParams>,
) {
    egui::Window::new("Tracks").show(ctx.ctx_mut(), |ui| {
        ui.add(egui::Slider::new(&mut params.radius, 2.5..=20.0).text("Radius"));
        ui.add_space(4.0);
        ui.label("Left-click to place.");
        ui.label("Right-click to cancel and erase.");
        ui.label("Hold Shift to allow S-bends.");
        ui.label("Scroll to zoom.");
        ui.label("Middle mouse to pan.");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let mut mut_state = state.0;
            ui.selectable_value(&mut mut_state, ControlState::None, "None");
            ui.selectable_value(&mut mut_state, ControlState::PlacingTracks, "Tracks");
            ui.selectable_value(&mut mut_state, ControlState::PlacingTrains, "Trains");
            if mut_state != state.0 {
                commands.insert_resource(NextState(mut_state));
            }
        });
    });
}
