use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// TODO: UNDO / REDO SYSTEM
// TODO: GRID PLACEMENT
// TODO: IMAGE LOADING

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

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SkillNodeData {
    id: u32,
    name: String,
    description: String,
    image_name: String,
    position: Vec2,
    node_type: NodeType,
    stats: Vec<StatModifier>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ConnectionData {
    from_id: u32,
    to_id: u32,
    #[serde(default)]
    control_points: Vec<Vec2>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SkillTreeSaveData {
    nodes: Vec<SkillNodeData>,
    connections: Vec<ConnectionData>,
    #[serde(default)]
    start_node_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
enum NodeType {
    Normal,
    Notable,
    Keystone,
    Start,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct StatModifier {
    stat_name: String,
    value: f32,
    modifier_type: ModifierType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
enum ModifierType {
    Flat,
    Percentage,
}

#[derive(Component)]
struct SkillNode {
    id: u32,
    data: SkillNodeData,
}

#[derive(Component)]
struct NodeVisual;

#[derive(Component)]
struct ConnectionVisual {
    from_id: u32,
    to_id: u32,
}

#[derive(Resource, Default)]
struct EditorState {
    file_path: Option<PathBuf>,
    show_save_dialog: bool,
    show_load_dialog: bool,
    save_path_buffer: String,
    load_path_buffer: String,
    next_node_id: u32,
}

#[derive(Resource, Default)]
struct SkillTreeData {
    nodes: HashMap<u32, Entity>,
    connections: Vec<ConnectionData>,
}

#[derive(Resource, Default)]
struct SelectedNode {
    entity: Option<Entity>,
    id: Option<u32>,
}

#[derive(Resource, Default)]
struct DragState {
    dragging: bool,
    offset: Vec2,
}

#[derive(Resource, Default)]
struct ConnectionMode {
    active: bool,
    start_node: Option<u32>,
}

#[derive(Resource)]
struct EditorCamera {
    zoom: f32,
    target_zoom: f32,
    pan_offset: Vec2,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            target_zoom: 1.0,
            pan_offset: Vec2::ZERO,
        }
    }
}

#[derive(Resource, Default)]
struct EguiInputState {
    wants_pointer_input: bool,
    wants_keyboard_input: bool,
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.1, 0.15)),
            ..default()
        },
    ));
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
        .lerp(editor_camera.target_zoom, 10.0 * time.delta_secs());

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
        if let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
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

                    let entity = spawn_node(&mut commands, &node_data);
                    skill_tree_data.nodes.insert(node_data.id, entity);
                    editor_state.next_node_id += 1;
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

    // If Shift is pressed, it might be a pan attempt, so don't do selection/deselection.
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
                // NOTE: CHANGE
                // Assuming node radius is 30.0 for clicking
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
                    let new_position = world_position + drag_state.offset;
                    transform.translation = new_position.extend(0.0);
                    node.data.position = new_position;
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
) {
    if egui_input_state.wants_keyboard_input {
        return;
    }

    // Delete currently selected node
    if keyboard.just_pressed(KeyCode::Backspace) {
        if let Some(node_id_to_delete) = selected_node.id {
            if let Some(entity_to_delete) = selected_node.entity {
                // Remove connections involving this node
                skill_tree_data
                    .connections
                    .retain(|conn| conn.from_id != node_id_to_delete && conn.to_id != node_id_to_delete);
                skill_tree_data.nodes.remove(&node_id_to_delete);

                commands.entity(entity_to_delete).despawn();

                selected_node.entity = None;
                selected_node.id = None;
            }
        }
    }
}

fn update_node_visuals(
    mut node_query: Query<(&SkillNode, &Children), Without<Text>>,
    mut sprite_query: Query<&mut Sprite>,
    selected_node: Res<SelectedNode>,
    connection_mode: Res<ConnectionMode>,
) {
    for (node, children) in node_query.iter_mut() {
        let is_selected = selected_node.id == Some(node.id);
        let is_connection_start =
            connection_mode.active && connection_mode.start_node == Some(node.id);

        for child in children.iter() {
            if let Ok(mut sprite) = sprite_query.get_mut(child) {
                sprite.color = if is_selected {
                    Color::srgb(0.3, 0.8, 0.3)
                } else if is_connection_start {
                    Color::srgb(0.8, 0.8, 0.3)
                } else {
                    match node.data.node_type {
                        NodeType::Normal => Color::srgb(0.5, 0.5, 0.6),
                        NodeType::Notable => Color::srgb(0.6, 0.5, 0.8),
                        NodeType::Keystone => Color::srgb(0.8, 0.5, 0.5),
                        NodeType::Start => Color::srgb(0.5, 0.8, 0.5),
                    }
                };
            }
        }
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

fn draw_grid(mut gizmos: Gizmos, editor_camera: Res<EditorCamera>) {
    let grid_size = 50.0;
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

fn ui_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut skill_tree_data: ResMut<SkillTreeData>,
    mut selected_node: ResMut<SelectedNode>,
    mut node_query: Query<&mut SkillNode>,
    mut commands: Commands,
    connection_mode: Res<ConnectionMode>,
) {
    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    for entity in skill_tree_data.nodes.values() {
                        commands.entity(*entity).despawn();
                    }
                    skill_tree_data.nodes.clear();
                    skill_tree_data.connections.clear();
                    selected_node.entity = None;
                    selected_node.id = None;
                    editor_state.file_path = None;
                    ui.close_menu();
                }

                if ui.button("Save").clicked() {
                    editor_state.show_save_dialog = true;
                    ui.close_menu();
                }

                if ui.button("Load").clicked() {
                    editor_state.show_load_dialog = true;
                    ui.close_menu();
                }
            });
        });
    });

    egui::SidePanel::left("properties_panel").show(ctx, |ui| {
        ui.heading("Skill Tree Editor");

        ui.separator();

        if connection_mode.active {
            ui.colored_label(egui::Color32::YELLOW, "Connection Mode Active");
            ui.label(format!(
                "Starting from node: {:?}",
                connection_mode.start_node
            ));
            ui.separator();
        }

        if let Some(entity) = selected_node.entity {
            if let Ok(mut node) = node_query.get_mut(entity) {
                ui.heading("Node Properties");

                ui.label(format!("ID: {}", node.id));

                ui.label("Name:");
                ui.text_edit_singleline(&mut node.data.name);

                ui.label("Description:");
                ui.text_edit_multiline(&mut node.data.description);

                ui.label("Image Name:");
                ui.text_edit_singleline(&mut node.data.image_name);

                ui.label("Node Type:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", node.data.node_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut node.data.node_type, NodeType::Normal, "Normal");
                        ui.selectable_value(&mut node.data.node_type, NodeType::Notable, "Notable");
                        ui.selectable_value(
                            &mut node.data.node_type,
                            NodeType::Keystone,
                            "Keystone",
                        );
                        ui.selectable_value(&mut node.data.node_type, NodeType::Start, "Start");
                    });

                ui.separator();
                ui.heading("Stats");

                let mut to_remove = None;
                for (i, stat) in node.data.stats.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut stat.stat_name);
                        ui.add(egui::DragValue::new(&mut stat.value).speed(0.1));

                        egui::ComboBox::from_id_salt(i)
                            .selected_text(format!("{:?}", stat.modifier_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut stat.modifier_type,
                                    ModifierType::Flat,
                                    "Flat",
                                );
                                ui.selectable_value(
                                    &mut stat.modifier_type,
                                    ModifierType::Percentage,
                                    "Percentage",
                                );
                            });

                        if ui.button("X").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }

                if let Some(index) = to_remove {
                    node.data.stats.remove(index);
                }

                if ui.button("Add Stat").clicked() {
                    node.data.stats.push(StatModifier {
                        stat_name: "New Stat".to_string(),
                        value: 0.0,
                        modifier_type: ModifierType::Flat,
                    });
                }

                ui.separator();

                if ui.button("Delete Node").clicked() {
                    let node_id = node.id;
                    skill_tree_data
                        .connections
                        .retain(|conn| conn.from_id != node_id && conn.to_id != node_id);

                    skill_tree_data.nodes.remove(&node_id);
                    commands.entity(entity).despawn();
                    selected_node.entity = None;
                    selected_node.id = None;
                }
            }
        } else {
            ui.label("No node selected");
            ui.separator();
            ui.label("Right-click to create a node");
            ui.label("Left-click to select a node");
            ui.label("Right-click on nodes to connect");
            ui.label("Middle mouse or Shift + Left Drag to pan");
            ui.label("Scroll to zoom");
        }

        ui.separator();
        ui.heading("Connections");

        let mut to_remove = None;
        for (i, connection) in skill_tree_data.connections.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{} -> {}", connection.from_id, connection.to_id));
                if ui.button("Remove").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        if let Some(index) = to_remove {
            skill_tree_data.connections.remove(index);
        }
    });

    if editor_state.show_save_dialog {
        egui::Window::new("Save Skill Tree")
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("File path:");
                ui.text_edit_singleline(&mut editor_state.save_path_buffer);

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        save_skill_tree(
                            &editor_state.save_path_buffer,
                            &skill_tree_data,
                            &node_query,
                        );
                        editor_state.file_path =
                            Some(PathBuf::from(&editor_state.save_path_buffer));
                        editor_state.show_save_dialog = false;
                    }

                    if ui.button("Cancel").clicked() {
                        editor_state.show_save_dialog = false;
                    }
                });
            });
    }

    if editor_state.show_load_dialog {
        egui::Window::new("Load Skill Tree")
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("File path:");
                ui.text_edit_singleline(&mut editor_state.load_path_buffer);

                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        if let Ok(save_data) = load_skill_tree(&editor_state.load_path_buffer) {
                            for entity in skill_tree_data.nodes.values() {
                                commands.entity(*entity).despawn();
                            }
                            skill_tree_data.nodes.clear();
                            skill_tree_data.connections.clear();

                            for node_data in save_data.nodes {
                                let entity = spawn_node(&mut commands, &node_data);
                                skill_tree_data.nodes.insert(node_data.id, entity);

                                if node_data.id >= editor_state.next_node_id {
                                    editor_state.next_node_id = node_data.id + 1;
                                }
                            }

                            skill_tree_data.connections = save_data.connections;
                            editor_state.file_path =
                                Some(PathBuf::from(&editor_state.load_path_buffer));
                        }
                        editor_state.show_load_dialog = false;
                    }

                    if ui.button("Cancel").clicked() {
                        editor_state.show_load_dialog = false;
                    }
                });
            });
    }
}

fn spawn_node(commands: &mut Commands, node_data: &SkillNodeData) -> Entity {
    let size = match node_data.node_type {
        NodeType::Normal => 40.0,
        NodeType::Notable => 50.0,
        NodeType::Keystone => 60.0,
        NodeType::Start => 55.0,
    };

    commands
        .spawn((
            SkillNode {
                id: node_data.id,
                data: node_data.clone(),
            },
            Transform::from_translation(node_data.position.extend(0.0)),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                NodeVisual,
                Sprite {
                    color: Color::srgb(0.5, 0.5, 0.6),
                    custom_size: Some(Vec2::splat(size)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(&node_data.name),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, -size * 0.8, 1.0)),
            ));
        })
        .id()
}

fn save_skill_tree(
    path: &str,
    skill_tree_data: &SkillTreeData,
    node_query: &Query<&mut SkillNode>,
) {
    let mut nodes = Vec::new();

    for node in node_query.iter() {
        nodes.push(node.data.clone());
    }

    let save_data = SkillTreeSaveData {
        nodes,
        connections: skill_tree_data.connections.clone(),
        start_node_id: None,
    };

    let ron_string = ron::ser::to_string_pretty(&save_data, Default::default()).unwrap();
    fs::write(path, ron_string).unwrap();
}

fn load_skill_tree(path: &str) -> Result<SkillTreeSaveData, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let save_data: SkillTreeSaveData = ron::from_str(&contents)?;
    Ok(save_data)
}
