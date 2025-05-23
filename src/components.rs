use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SkillNodeData {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub image_name: String,
    pub position: Vec2,
    pub node_type: NodeType,
    pub stats: Vec<StatModifier>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionData {
    pub from_id: u32,
    pub to_id: u32,
    #[serde(default)]
    pub curve_type: CurveType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CurveType {
    Straight,
    Arc { radius: f32, clockwise: bool },
}

impl Default for CurveType {
    fn default() -> Self {
        CurveType::Straight
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SkillTreeSaveData {
    pub nodes: Vec<SkillNodeData>,
    pub connections: Vec<ConnectionData>,
    #[serde(default)]
    pub start_node_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum NodeType {
    Normal,
    Notable,
    Keystone,
    Start,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatModifier {
    pub stat_name: String,
    pub value: f32,
    pub modifier_type: ModifierType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ModifierType {
    Flat,
    Percentage,
}

#[derive(Component)]
pub struct SkillNode {
    pub id: u32,
    pub data: SkillNodeData,
}

#[derive(Component)]
pub struct ConnectionVisual {
    pub from_id: u32,
    pub to_id: u32,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum NextActionAfterSaveAs {
    #[default]
    None,
    ShowLoadDialog,
    CreateNewFile,
}

#[derive(Resource, Default)]
pub struct EditorState {
    pub current_file_path: Option<PathBuf>,
    pub show_save_as_dialog: bool,
    pub show_load_dialog: bool,
    pub save_as_file_name_buffer: String,
    pub available_ron_files: Vec<PathBuf>,
    pub next_node_id: u32,
    pub save_as_conflict_path: Option<PathBuf>,
    pub save_as_show_overwrite_prompt: bool,
    pub dirty: bool,
    pub show_unsaved_changes_on_load_dialog: bool,
    pub show_unsaved_changes_on_new_dialog: bool,
    pub next_action_after_save_as: NextActionAfterSaveAs,
    pub trigger_pending_action: NextActionAfterSaveAs,
}

#[derive(Resource, Default)]
pub struct NodeImages {
    pub skill_node: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct GridSettings {
    pub snap_to_grid: bool,
    pub grid_size: f32,
}

#[derive(Resource, Default)]
pub struct SkillTreeData {
    pub nodes: HashMap<u32, Entity>,
    pub connections: Vec<ConnectionData>,
}

#[derive(Resource, Default)]
pub struct SelectedNode {
    pub entity: Option<Entity>,
    pub id: Option<u32>,
}

#[derive(Resource, Default)]
pub struct SelectedConnection {
    pub index: Option<usize>,
}

#[derive(Resource, Default)]
pub struct DragState {
    pub dragging: bool,
    pub offset: Vec2,
}

#[derive(Resource, Default)]
pub struct ConnectionMode {
    pub active: bool,
    pub start_node: Option<u32>,
}

#[derive(Resource)]
pub struct EditorCamera {
    pub zoom: f32,
    pub target_zoom: f32,
    pub pan_offset: Vec2,
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
pub struct EguiInputState {
    pub wants_pointer_input: bool,
    pub wants_keyboard_input: bool,
}
