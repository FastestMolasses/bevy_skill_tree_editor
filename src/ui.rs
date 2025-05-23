use super::spawn_node;
use crate::components::*;
use crate::fs::{load_skill_tree, save_skill_tree};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use std::path::PathBuf;
use std::{fs, mem};

pub fn ui_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut skill_tree_data: ResMut<SkillTreeData>,
    mut selected_node: ResMut<SelectedNode>,
    mut selected_connection: ResMut<SelectedConnection>,
    mut node_query: Query<&mut SkillNode>,
    mut commands: Commands,
    connection_mode: Res<ConnectionMode>,
    mut grid_settings: ResMut<GridSettings>,
    node_images: Res<NodeImages>,
) {
    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    if editor_state.dirty {
                        editor_state.show_unsaved_changes_on_new_dialog = true;
                        editor_state.next_action_after_save_as = NextActionAfterSaveAs::None;
                    } else {
                        perform_new_file_action(
                            &mut commands,
                            &mut editor_state,
                            &mut skill_tree_data,
                            &mut selected_node,
                            &mut selected_connection,
                        );
                    }
                    ui.close_menu();
                }

                if ui.button("Save").clicked() {
                    if let Some(path) = editor_state.current_file_path.clone() {
                        save_skill_tree(
                            path.to_str().unwrap_or("skill_tree.ron"),
                            &skill_tree_data,
                            &node_query,
                        );
                        editor_state.dirty = false;
                    } else {
                        editor_state.save_as_file_name_buffer = editor_state
                            .current_file_path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|os_str| os_str.to_str())
                            .unwrap_or("untitled.ron")
                            .to_string();
                        editor_state.show_save_as_dialog = true;
                        editor_state.save_as_show_overwrite_prompt = false;
                        editor_state.save_as_conflict_path = None;
                    }
                    ui.close_menu();
                }

                if ui.button("Save As...").clicked() {
                    editor_state.save_as_file_name_buffer = editor_state
                        .current_file_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|os_str| os_str.to_str())
                        .unwrap_or("untitled.ron")
                        .to_string();
                    editor_state.show_save_as_dialog = true;
                    editor_state.save_as_show_overwrite_prompt = false;
                    editor_state.save_as_conflict_path = None;
                    editor_state.next_action_after_save_as = NextActionAfterSaveAs::None;
                    ui.close_menu();
                }

                if ui.button("Load").clicked() {
                    if editor_state.dirty {
                        editor_state.show_unsaved_changes_on_load_dialog = true;
                        editor_state.next_action_after_save_as = NextActionAfterSaveAs::None;
                    } else {
                        open_load_dialog_sequence(&mut editor_state);
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button("View", |ui| {
                if ui
                    .checkbox(&mut grid_settings.snap_to_grid, "Snap to Grid")
                    .clicked()
                {
                    ui.close_menu();
                }
            });
        });
    });

    egui::SidePanel::left("properties_panel").show(ctx, |ui| {
        ui.heading("Skill Tree Editor");
        ui.separator();
        ui.checkbox(&mut grid_settings.snap_to_grid, "Snap to Grid");
        ui.add(egui::Slider::new(&mut grid_settings.grid_size, 10.0..=200.0).text("Grid Size"));
        ui.separator();

        if connection_mode.active {
            ui.colored_label(egui::Color32::YELLOW, "Connection Mode Active");
            ui.label(format!(
                "Starting from node: {:?}",
                connection_mode.start_node
            ));
            ui.separator();
        }

        // Connection Properties
        if let Some(connection_index) = selected_connection.index {
            if let Some(connection) = skill_tree_data.connections.get_mut(connection_index) {
                ui.heading("Connection Properties");
                ui.label(format!(
                    "From Node {} to Node {}",
                    connection.from_id, connection.to_id
                ));

                ui.separator();
                ui.label("Curve Type:");

                let mut curve_type_changed = false;
                let is_straight = matches!(connection.curve_type, CurveType::Straight);

                if ui.radio(is_straight, "Straight").clicked() {
                    connection.curve_type = CurveType::Straight;
                    curve_type_changed = true;
                }

                if ui.radio(!is_straight, "Arc").clicked() {
                    connection.curve_type = CurveType::Arc {
                        radius: 100.0,
                        clockwise: false,
                    };
                    curve_type_changed = true;
                }

                if let CurveType::Arc {
                    ref mut radius,
                    ref mut clockwise,
                } = connection.curve_type
                {
                    ui.separator();
                    ui.label("Arc Properties:");

                    if ui
                        .add(egui::Slider::new(radius, 30.0..=500.0).text("Radius"))
                        .changed()
                    {
                        curve_type_changed = true;
                    }

                    if ui.checkbox(clockwise, "Clockwise").changed() {
                        curve_type_changed = true;
                    }

                    // Check if arc is valid and show warning
                    let mut from_pos = None;
                    let mut to_pos = None;
                    for node in node_query.iter() {
                        if node.id == connection.from_id {
                            from_pos = Some(node.data.position);
                        }
                        if node.id == connection.to_id {
                            to_pos = Some(node.data.position);
                        }
                    }

                    if let (Some(from), Some(to)) = (from_pos, to_pos) {
                        let distance = from.distance(to);
                        let min_radius = distance / 2.0;

                        if *radius < min_radius {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 200, 100),
                                format!("⚠ Radius too small! Minimum: {:.1}", min_radius),
                            );
                            ui.label("Arc will display as dashed line");

                            if ui.button("Auto-fix radius").clicked() {
                                *radius = min_radius + 10.0; // Add a small buffer
                                curve_type_changed = true;
                            }
                        }
                    }

                    ui.add_space(5.0);
                    ui.label("Tips:");
                    ui.label("• Larger radius = gentler curve");
                    ui.label("• Toggle clockwise to flip the arc");
                }

                if curve_type_changed {
                    editor_state.dirty = true;
                }

                ui.separator();
                if ui.button("Delete Connection").clicked() {
                    skill_tree_data.connections.remove(connection_index);
                    selected_connection.index = None;
                    editor_state.dirty = true;
                }
            }
        }
        // Node Properties
        else if let Some(entity) = selected_node.entity {
            if let Ok(mut node) = node_query.get_mut(entity) {
                ui.heading("Node Properties");
                ui.label(format!("ID: {}", node.id));
                ui.label("Name:");
                if ui.text_edit_singleline(&mut node.data.name).changed() {
                    editor_state.dirty = true;
                }
                ui.label("Description:");
                if ui.text_edit_multiline(&mut node.data.description).changed() {
                    editor_state.dirty = true;
                }
                ui.label("Image Name:");
                if ui.text_edit_singleline(&mut node.data.image_name).changed() {
                    editor_state.dirty = true;
                }

                ui.label("Node Type:");
                let mut node_type_changed = false;
                egui::ComboBox::from_label("NodeType")
                    .selected_text(format!("{:?}", node.data.node_type))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut node.data.node_type, NodeType::Normal, "Normal")
                            .clicked()
                        {
                            node_type_changed = true;
                        }
                        if ui
                            .selectable_value(
                                &mut node.data.node_type,
                                NodeType::Notable,
                                "Notable",
                            )
                            .clicked()
                        {
                            node_type_changed = true;
                        }
                        if ui
                            .selectable_value(
                                &mut node.data.node_type,
                                NodeType::Keystone,
                                "Keystone",
                            )
                            .clicked()
                        {
                            node_type_changed = true;
                        }
                        if ui
                            .selectable_value(&mut node.data.node_type, NodeType::Start, "Start")
                            .clicked()
                        {
                            node_type_changed = true;
                        }
                    });
                if node_type_changed {
                    editor_state.dirty = true;
                }

                ui.separator();
                ui.heading("Stats");
                let mut stat_to_remove_idx = None;
                for (i, stat) in node.data.stats.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.text_edit_singleline(&mut stat.stat_name).changed() {
                            editor_state.dirty = true;
                        }
                        if ui
                            .add(egui::DragValue::new(&mut stat.value).speed(0.1))
                            .changed()
                        {
                            editor_state.dirty = true;
                        }

                        let mut mod_type_changed = false;
                        egui::ComboBox::from_id_salt(format!("mod_type_{i}"))
                            .selected_text(format!("{:?}", stat.modifier_type))
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_value(
                                        &mut stat.modifier_type,
                                        ModifierType::Flat,
                                        "Flat",
                                    )
                                    .clicked()
                                {
                                    mod_type_changed = true;
                                }
                                if ui
                                    .selectable_value(
                                        &mut stat.modifier_type,
                                        ModifierType::Percentage,
                                        "Percentage",
                                    )
                                    .clicked()
                                {
                                    mod_type_changed = true;
                                }
                            });
                        if mod_type_changed {
                            editor_state.dirty = true;
                        }

                        if ui.button("X").clicked() {
                            stat_to_remove_idx = Some(i);
                            editor_state.dirty = true;
                        }
                    });
                }
                if let Some(index) = stat_to_remove_idx {
                    node.data.stats.remove(index);
                }
                if ui.button("Add Stat").clicked() {
                    node.data.stats.push(StatModifier {
                        stat_name: "New Stat".to_string(),
                        value: 0.0,
                        modifier_type: ModifierType::Flat,
                    });
                    editor_state.dirty = true;
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
                    editor_state.dirty = true;
                }
            }
        } else {
            ui.label("No node or connection selected");
            ui.separator();
            ui.label("Right-click to create a node");
            ui.label("Left-click to select nodes/connections");
            ui.label("Right-click on nodes to connect");
            ui.label("Middle mouse or Shift + Left Drag to pan");
            ui.label("Scroll to zoom");
            ui.label("Delete/Backspace to remove selected");
        }
        ui.separator();
        ui.heading("All Connections");

        // Use available height for the scroll area
        let available_height = ui.available_height();

        egui::ScrollArea::vertical()
            .max_height(available_height)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                let mut connection_to_remove_idx = None;
                for (i, connection) in skill_tree_data.connections.iter().enumerate() {
                    // Use a full-width group to make the entire row scrollable
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            // Check if arc is valid
                            let mut is_invalid_arc = false;
                            if let CurveType::Arc { radius, .. } = &connection.curve_type {
                                let mut from_pos = None;
                                let mut to_pos = None;
                                for node in node_query.iter() {
                                    if node.id == connection.from_id {
                                        from_pos = Some(node.data.position);
                                    }
                                    if node.id == connection.to_id {
                                        to_pos = Some(node.data.position);
                                    }
                                }
                                if let (Some(from), Some(to)) = (from_pos, to_pos) {
                                    let distance = from.distance(to);
                                    is_invalid_arc = *radius < distance / 2.0;
                                }
                            }

                            let connection_text = match &connection.curve_type {
                                CurveType::Straight => {
                                    format!("{} → {}", connection.from_id, connection.to_id)
                                }
                                CurveType::Arc { .. } => {
                                    if is_invalid_arc {
                                        format!("{} ⚠ {}", connection.from_id, connection.to_id)
                                    } else {
                                        format!("{} ⤷ {}", connection.from_id, connection.to_id)
                                    }
                                }
                            };

                            let selected = selected_connection.index == Some(i);
                            let button = if selected {
                                ui.add(
                                    egui::Button::new(connection_text)
                                        .fill(egui::Color32::from_rgb(60, 80, 100)),
                                )
                            } else {
                                ui.button(connection_text)
                            };

                            if button.clicked() {
                                selected_connection.index = Some(i);
                                selected_node.entity = None;
                                selected_node.id = None;
                            }

                            if ui.button("×").clicked() {
                                connection_to_remove_idx = Some(i);
                                editor_state.dirty = true;
                            }
                        });
                    });
                }

                if let Some(index) = connection_to_remove_idx {
                    skill_tree_data.connections.remove(index);
                    if selected_connection.index == Some(index) {
                        selected_connection.index = None;
                    } else if selected_connection.index.is_some()
                        && selected_connection.index.unwrap() > index
                    {
                        // Adjust selected index if a connection before it was removed
                        selected_connection.index = Some(selected_connection.index.unwrap() - 1);
                    }
                }

                ui.add_space(10.0);
            });
    });

    if editor_state.show_save_as_dialog {
        egui::Window::new("Save Skill Tree As...")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("File name:");
                let filename_input_response =
                    ui.text_edit_singleline(&mut editor_state.save_as_file_name_buffer);

                if filename_input_response.changed() {
                    editor_state.save_as_show_overwrite_prompt = false;
                    editor_state.save_as_conflict_path = None;
                }

                if editor_state.save_as_show_overwrite_prompt {
                    if let Some(conflicting_path) = &editor_state.save_as_conflict_path {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!(
                                "Error: File '{}' already exists!",
                                conflicting_path.display()
                            ),
                        );
                    } else {
                        ui.colored_label(egui::Color32::RED, "Error: File already exists!");
                    }
                }

                ui.horizontal(|ui| {
                    let save_as_show_overwrite_prompt = editor_state.save_as_show_overwrite_prompt;
                    let save_as_file_name_buffer_clone =
                        editor_state.save_as_file_name_buffer.clone();

                    let mut attempt_save_action = |es: &mut EditorState, path_to_save: PathBuf| {
                        save_skill_tree(
                            path_to_save.to_str().unwrap_or_default(),
                            &skill_tree_data,
                            &node_query,
                        );
                        es.current_file_path = Some(path_to_save.clone());
                        es.dirty = false;
                        es.show_save_as_dialog = false;
                        es.save_as_show_overwrite_prompt = false;
                        es.save_as_conflict_path = None;

                        es.trigger_pending_action = es.next_action_after_save_as;
                        es.next_action_after_save_as = NextActionAfterSaveAs::None;
                    };

                    if save_as_show_overwrite_prompt {
                        ui.add_enabled(false, egui::Button::new("Save"));
                    } else if ui.button("Save").clicked()
                        && !save_as_file_name_buffer_clone.is_empty()
                    {
                        let mut path_for_saving = PathBuf::from(&save_as_file_name_buffer_clone);
                        if path_for_saving.extension().is_none_or(|ext| ext != "ron") {
                            path_for_saving.set_extension("ron");
                        }

                        if path_for_saving.exists() {
                            editor_state.save_as_show_overwrite_prompt = true;
                            editor_state.save_as_conflict_path = Some(path_for_saving);
                        } else {
                            attempt_save_action(&mut editor_state, path_for_saving);
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        editor_state.show_save_as_dialog = false;
                        editor_state.save_as_show_overwrite_prompt = false;
                        editor_state.save_as_conflict_path = None;
                        editor_state.next_action_after_save_as = NextActionAfterSaveAs::None;
                    }

                    if editor_state.save_as_show_overwrite_prompt {
                        if let Some(path_to_overwrite) = editor_state.save_as_conflict_path.clone()
                        {
                            if ui.button("Overwrite").clicked() {
                                attempt_save_action(&mut editor_state, path_to_overwrite);
                            }
                        }
                    }
                });
            });
    }

    if editor_state.show_unsaved_changes_on_new_dialog {
        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("You have unsaved changes. Starting a new file will discard them. What would you like to do?");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if let Some(path) = editor_state.current_file_path.clone() {
                            save_skill_tree(
                                path.to_str().unwrap_or("skill_tree.ron"),
                                &skill_tree_data,
                                &node_query,
                            );
                            perform_new_file_action(&mut commands, &mut editor_state, &mut skill_tree_data, &mut selected_node, &mut selected_connection);
                            editor_state.show_unsaved_changes_on_new_dialog = false;
                        } else {
                            editor_state.next_action_after_save_as = NextActionAfterSaveAs::CreateNewFile;
                            editor_state.save_as_file_name_buffer = editor_state
                                .current_file_path
                                .as_ref()
                                .and_then(|p| p.file_name())
                                .and_then(|os_str| os_str.to_str())
                                .unwrap_or("untitled.ron")
                                .to_string();
                            editor_state.show_save_as_dialog = true;
                            editor_state.save_as_show_overwrite_prompt = false;
                            editor_state.save_as_conflict_path = None;
                            editor_state.show_unsaved_changes_on_new_dialog = false;
                        }
                    }
                    if ui.button("Don't Save").clicked() {
                        perform_new_file_action(&mut commands, &mut editor_state, &mut skill_tree_data, &mut selected_node, &mut selected_connection);
                        editor_state.show_unsaved_changes_on_new_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        editor_state.show_unsaved_changes_on_new_dialog = false;
                        editor_state.next_action_after_save_as = NextActionAfterSaveAs::None;
                    }
                });
            });
    }

    if editor_state.show_unsaved_changes_on_load_dialog {
        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("You have unsaved changes. Loading a file will discard them. What would you like to do?");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if let Some(path) = editor_state.current_file_path.clone() {
                            save_skill_tree(
                                path.to_str().unwrap_or("skill_tree.ron"),
                                &skill_tree_data,
                                &node_query,
                            );
                            editor_state.dirty = false;
                            open_load_dialog_sequence(&mut editor_state);
                            editor_state.show_unsaved_changes_on_load_dialog = false;
                        } else {
                            editor_state.next_action_after_save_as =
                                NextActionAfterSaveAs::ShowLoadDialog;
                            editor_state.save_as_file_name_buffer = editor_state
                                .current_file_path
                                .as_ref()
                                .and_then(|p| p.file_name())
                                .and_then(|os_str| os_str.to_str())
                                .unwrap_or("untitled.ron")
                                .to_string();
                            editor_state.show_save_as_dialog = true;
                            editor_state.save_as_show_overwrite_prompt = false;
                            editor_state.save_as_conflict_path = None;
                            editor_state.show_unsaved_changes_on_load_dialog = false;
                        }
                    }
                    if ui.button("Don't Save").clicked() {
                        editor_state.dirty = false;
                        open_load_dialog_sequence(&mut editor_state);
                        editor_state.show_unsaved_changes_on_load_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        editor_state.show_unsaved_changes_on_load_dialog = false;
                        editor_state.next_action_after_save_as = NextActionAfterSaveAs::None;
                    }
                });
            });
    }

    if editor_state.show_load_dialog {
        egui::Window::new("Load Skill Tree")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading("Select a .ron file to load:");
                ui.separator();
                let mut file_to_load_and_close_dialog = None;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for path_buf in &editor_state.available_ron_files {
                        if ui
                            .button(path_buf.file_name().unwrap_or_default().to_string_lossy())
                            .clicked()
                        {
                            file_to_load_and_close_dialog = Some(path_buf.clone());
                        }
                    }
                });

                if let Some(path_to_load) = file_to_load_and_close_dialog {
                    if let Ok(save_data) =
                        load_skill_tree(path_to_load.to_str().unwrap_or_default())
                    {
                        // Clear existing tree before loading new one
                        perform_new_file_action(
                            &mut commands,
                            &mut editor_state,
                            &mut skill_tree_data,
                            &mut selected_node,
                            &mut selected_connection,
                        );

                        let mut max_id = 0;
                        for node_data in save_data.nodes {
                            let entity = spawn_node(&mut commands, &node_data, &node_images);
                            skill_tree_data.nodes.insert(node_data.id, entity);
                            if node_data.id >= max_id {
                                max_id = node_data.id + 1;
                            }
                        }
                        editor_state.next_node_id = max_id;
                        skill_tree_data.connections = save_data.connections;
                        editor_state.current_file_path = Some(path_to_load);
                        editor_state.dirty = false; // Loaded file is not dirty
                    }
                    editor_state.show_load_dialog = false;
                }
                ui.separator();
                if ui.button("Cancel").clicked() {
                    editor_state.show_load_dialog = false;
                }
            });
    }

    let action_to_trigger = mem::replace(
        &mut editor_state.trigger_pending_action,
        NextActionAfterSaveAs::None,
    );
    match action_to_trigger {
        NextActionAfterSaveAs::ShowLoadDialog => {
            open_load_dialog_sequence(&mut editor_state);
        }
        NextActionAfterSaveAs::CreateNewFile => {
            perform_new_file_action(
                &mut commands,
                &mut editor_state,
                &mut skill_tree_data,
                &mut selected_node,
                &mut selected_connection,
            );
        }
        NextActionAfterSaveAs::None => {}
    }
}

fn open_load_dialog_sequence(editor_state: &mut EditorState) {
    editor_state.available_ron_files.clear();
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "ron") {
                editor_state.available_ron_files.push(path);
            }
        }
    }
    editor_state.available_ron_files.sort();
    editor_state.show_load_dialog = true;
}

fn perform_new_file_action(
    commands: &mut Commands,
    editor_state: &mut EditorState,
    skill_tree_data: &mut SkillTreeData,
    selected_node: &mut SelectedNode,
    selected_connection: &mut SelectedConnection,
) {
    for entity in skill_tree_data.nodes.values() {
        commands.entity(*entity).despawn();
    }
    skill_tree_data.nodes.clear();
    skill_tree_data.connections.clear();
    selected_node.entity = None;
    selected_node.id = None;
    selected_connection.index = None;
    editor_state.current_file_path = None;
    editor_state.next_node_id = 0;
    editor_state.dirty = false;
}
