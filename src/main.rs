mod components;
mod fs;
mod ui;

use crate::components::*;
use crate::ui::ui_system;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};

// TODO: UNDO / REDO SYSTEM
// TODO: ADD CONTROL POINTS FOR CONNECTIONS

const GRID_SIZE: f32 = 50.0;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin {
                enable_multipass_for_primary_context: false,
            },
        ))
        .init_resource::<EditorState>()
        .init_resource::<SkillTreeData>()
        .init_resource::<SelectedNode>()
        .init_resource::<DragState>()
        .init_resource::<ConnectionMode>()
        .init_resource::<EditorCamera>()
        .init_resource::<EguiInputState>()
        .init_resource::<GridSettings>()
        .init_resource::<NodeImages>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                ui_system,
                update_egui_input_state.after(ui_system),
                (
                    update_camera,
                    handle_mouse_input,
                    handle_node_selection,
                    handle_node_dragging,
                    update_node_visuals,
                    draw_connections,
                    draw_grid,
                    handle_keyboard_shortcuts,
                )
                    .after(update_egui_input_state),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut grid_settings: ResMut<GridSettings>,
    mut node_images: ResMut<NodeImages>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.1, 0.15)),
            ..default()
        },
    ));
    grid_settings.grid_size = GRID_SIZE;
    grid_settings.snap_to_grid = true;

    node_images.skill_node = asset_server.load("skill_border_01.png");
}

fn update_egui_input_state(
    mut egui_contexts: EguiContexts,
    mut egui_input_state: ResMut<EguiInputState>,
) {
    if let Some(ctx) = egui_contexts.try_ctx_mut() {
        egui_input_state.wants_pointer_input = ctx.wants_pointer_input();
        egui_input_state.wants_keyboard_input = ctx.wants_keyboard_input();
    }
}

fn update_camera(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    mut editor_camera: ResMut<EditorCamera>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<CursorMoved>,
    mut mouse_wheel: EventReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    egui_input_state: Res<EguiInputState>,
) {
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    if egui_input_state.wants_pointer_input {
        mouse_wheel.clear();
        mouse_motion.clear();
        return;
    }

    for event in mouse_wheel.read() {
        editor_camera.target_zoom *= 1.0 - event.y * 0.1;
        editor_camera.target_zoom = editor_camera.target_zoom.clamp(0.1, 5.0);
    }

    editor_camera.zoom = editor_camera
        .zoom
        .lerp(editor_camera.target_zoom, 6.0 * time.delta_secs());

    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if mouse_button.pressed(MouseButton::Middle)
        || (shift_pressed && mouse_button.pressed(MouseButton::Left))
    {
        let mut pan_input_delta = Vec2::ZERO;
        for event in mouse_motion.read() {
            if let Some(e_delta) = event.delta {
                pan_input_delta.x -= e_delta.x;
                pan_input_delta.y += e_delta.y;
            }
        }
        let zoom = editor_camera.zoom;
        editor_camera.pan_offset += pan_input_delta * zoom;
    } else {
        mouse_motion.clear();
    }

    if !egui_input_state.wants_keyboard_input {
        let pan_speed = 500.0 * time.delta_secs() * editor_camera.zoom;
        if keyboard.pressed(KeyCode::ArrowLeft) {
            editor_camera.pan_offset.x -= pan_speed;
        }
        if keyboard.pressed(KeyCode::ArrowRight) {
            editor_camera.pan_offset.x += pan_speed;
        }
        if keyboard.pressed(KeyCode::ArrowUp) {
            editor_camera.pan_offset.y += pan_speed;
        }
        if keyboard.pressed(KeyCode::ArrowDown) {
            editor_camera.pan_offset.y -= pan_speed;
        }
    }

    camera_transform.scale = Vec3::splat(editor_camera.zoom);
    camera_transform.translation = editor_camera
        .pan_offset
        .extend(camera_transform.translation.z);
}

fn snap_to_grid_logic(position: Vec2, grid_size: f32) -> Vec2 {
    Vec2::new(
        (position.x / grid_size).round() * grid_size,
        (position.y / grid_size).round() * grid_size,
    )
}

fn handle_mouse_input(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut editor_state: ResMut<EditorState>,
    mut skill_tree_data: ResMut<SkillTreeData>,
    editor_camera: Res<EditorCamera>,
    selected_node: Res<SelectedNode>,
    mut connection_mode: ResMut<ConnectionMode>,
    node_query: Query<(Entity, &SkillNode, &Transform)>,
    egui_input_state: Res<EguiInputState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    grid_settings: Res<GridSettings>,
    node_images: Res<NodeImages>,
) {
    if egui_input_state.wants_pointer_input {
        return;
    }

    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if shift_pressed && mouse_button.pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    if let Some(cursor_position) = window.cursor_position() {
        if let Ok(mut world_position) =
            camera.viewport_to_world_2d(camera_transform, cursor_position)
        {
            if grid_settings.snap_to_grid {
                world_position = snap_to_grid_logic(world_position, grid_settings.grid_size);
            }

            if mouse_button.just_pressed(MouseButton::Right) {
                let mut clicked_node = None;
                for (entity, node, transform) in node_query.iter() {
                    let distance = world_position.distance(transform.translation.xy());
                    if distance < 30.0 {
                        clicked_node = Some(node.id);
                        break;
                    }
                }

                if let Some(node_id) = clicked_node {
                    if connection_mode.active && connection_mode.start_node.is_some() {
                        let start_id = connection_mode.start_node.unwrap();
                        if start_id != node_id {
                            skill_tree_data.connections.push(ConnectionData {
                                from_id: start_id,
                                to_id: node_id,
                                control_points: vec![],
                            });
                            editor_state.dirty = true;
                        }
                        connection_mode.active = false;
                        connection_mode.start_node = None;
                    } else {
                        connection_mode.active = true;
                        connection_mode.start_node = Some(node_id);
                    }
                } else if !connection_mode.active {
                    let node_data = SkillNodeData {
                        id: editor_state.next_node_id,
                        name: format!("Node {}", editor_state.next_node_id),
                        description: "Node description".to_string(),
                        image_name: "default_node.png".to_string(),
                        position: world_position,
                        node_type: NodeType::Normal,
                        stats: vec![],
                    };

                    let entity = spawn_node(&mut commands, &node_data, &node_images);
                    skill_tree_data.nodes.insert(node_data.id, entity);
                    editor_state.next_node_id += 1;
                    editor_state.dirty = true;
                } else {
                    connection_mode.active = false;
                    connection_mode.start_node = None;
                }
            }
        }
    }
}

fn handle_node_selection(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    node_query: Query<(Entity, &SkillNode, &Transform)>,
    mut selected_node: ResMut<SelectedNode>,
    mut drag_state: ResMut<DragState>,
    egui_input_state: Res<EguiInputState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    if egui_input_state.wants_pointer_input {
        return;
    }

    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if shift_pressed {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    if let Some(cursor_position) = window.cursor_position() {
        if let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
            let mut closest_node = None;
            let mut closest_distance = f32::MAX;

            for (entity, node, transform) in node_query.iter() {
                let distance = world_position.distance(transform.translation.xy());
                if distance < 30.0 && distance < closest_distance {
                    closest_distance = distance;
                    closest_node = Some((entity, node.id, transform.translation.xy()));
                }
            }

            if let Some((entity, id, node_pos)) = closest_node {
                selected_node.entity = Some(entity);
                selected_node.id = Some(id);
                drag_state.dragging = true;
                drag_state.offset = node_pos - world_position;
            } else {
                selected_node.entity = None;
                selected_node.id = None;
            }
        }
    }
}

fn handle_node_dragging(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut node_query: Query<(&mut Transform, &mut SkillNode)>,
    selected_node: Res<SelectedNode>,
    mut drag_state: ResMut<DragState>,
    egui_input_state: Res<EguiInputState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    grid_settings: Res<GridSettings>,
    mut editor_state: ResMut<EditorState>,
) {
    if !drag_state.dragging {
        return;
    }

    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if shift_pressed && mouse_button.pressed(MouseButton::Left) {
        drag_state.dragging = false;
        return;
    }

    if mouse_button.just_released(MouseButton::Left) {
        drag_state.dragging = false;
        return;
    }

    if egui_input_state.wants_pointer_input {
        drag_state.dragging = false;
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    if let Some(entity) = selected_node.entity {
        if let Some(cursor_position) = window.cursor_position() {
            if let Ok(world_position) =
                camera.viewport_to_world_2d(camera_transform, cursor_position)
            {
                if let Ok((mut transform, mut node)) = node_query.get_mut(entity) {
                    let mut new_position = world_position + drag_state.offset;
                    if grid_settings.snap_to_grid {
                        new_position = snap_to_grid_logic(new_position, grid_settings.grid_size);
                    }
                    transform.translation = new_position.extend(0.0);
                    node.data.position = new_position;
                    editor_state.dirty = true;
                }
            }
        }
    }
}

fn handle_keyboard_shortcuts(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected_node: ResMut<SelectedNode>,
    mut skill_tree_data: ResMut<SkillTreeData>,
    egui_input_state: Res<EguiInputState>,
    mut editor_state: ResMut<EditorState>,
) {
    if egui_input_state.wants_keyboard_input {
        return;
    }

    if keyboard.just_pressed(KeyCode::Backspace) {
        if let Some(node_id_to_delete) = selected_node.id {
            if let Some(entity_to_delete) = selected_node.entity {
                skill_tree_data.connections.retain(|conn| {
                    conn.from_id != node_id_to_delete && conn.to_id != node_id_to_delete
                });
                skill_tree_data.nodes.remove(&node_id_to_delete);

                commands.entity(entity_to_delete).despawn();

                selected_node.entity = None;
                selected_node.id = None;
                editor_state.dirty = true;
            }
        }
    }
}

fn update_node_visuals(
    mut node_query: Query<(&SkillNode, &mut Sprite)>,
    selected_node: Res<SelectedNode>,
    connection_mode: Res<ConnectionMode>,
) {
    for (node, mut sprite) in node_query.iter_mut() {
        let is_selected = selected_node.id == Some(node.id);
        let is_connection_start =
            connection_mode.active && connection_mode.start_node == Some(node.id);

        sprite.color = if is_connection_start {
            Color::srgb(0.3, 0.5, 0.8)
        } else if is_selected {
            Color::srgb(0.3, 0.8, 0.4)
        } else {
            Color::srgb(1.0, 1.0, 1.0)
        };
    }
}

fn draw_connections(
    mut gizmos: Gizmos,
    skill_tree_data: Res<SkillTreeData>,
    node_query: Query<(&SkillNode, &Transform)>,
) {
    for connection in &skill_tree_data.connections {
        let mut from_pos = None;
        let mut to_pos = None;

        for (node, transform) in node_query.iter() {
            if node.id == connection.from_id {
                from_pos = Some(transform.translation.xy());
            }
            if node.id == connection.to_id {
                to_pos = Some(transform.translation.xy());
            }
        }

        if let (Some(from), Some(to)) = (from_pos, to_pos) {
            gizmos.line_2d(from, to, Color::srgb(0.7, 0.6, 0.4));
        }
    }
}

fn draw_grid(mut gizmos: Gizmos, grid_settings: Res<GridSettings>) {
    if !grid_settings.snap_to_grid {
        return;
    }
    let grid_size = grid_settings.grid_size;
    let grid_count = 50;
    let half_size = (grid_count as f32 * grid_size) / 2.0;

    let color = Color::srgba(0.3, 0.3, 0.3, 0.2);

    for i in 0..=grid_count {
        let x = -half_size + (i as f32 * grid_size);
        gizmos.line_2d(Vec2::new(x, -half_size), Vec2::new(x, half_size), color);
    }

    for i in 0..=grid_count {
        let y = -half_size + (i as f32 * grid_size);
        gizmos.line_2d(Vec2::new(-half_size, y), Vec2::new(half_size, y), color);
    }
}

pub fn spawn_node(
    commands: &mut Commands,
    node_data: &SkillNodeData,
    node_images: &NodeImages,
) -> Entity {
    commands
        .spawn((
            SkillNode {
                id: node_data.id,
                data: node_data.clone(),
            },
            Transform::from_translation(node_data.position.extend(0.0)),
            Sprite {
                custom_size: Some(Vec2::splat(60.0)),
                image: node_images.skill_node.clone(),
                ..default()
            },
        ))
        .id()
}
