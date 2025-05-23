use std::fs;
use bevy::prelude::*;
use crate::components::*;

pub fn save_skill_tree(
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
    if path.is_empty() {
        warn!("Attempted to save with an empty path. Save operation cancelled.");
        return;
    }
    if let Err(e) = fs::write(path, ron_string) {
        error!("Failed to save skill tree to {}: {}", path, e);
    } else {
        info!("Skill tree saved to {}", path);
    }
}

pub fn load_skill_tree(path: &str) -> Result<SkillTreeSaveData, Box<dyn std::error::Error>> {
    if path.is_empty() {
        return Err("Load path is empty".into());
    }
    let contents = fs::read_to_string(path)?;
    let save_data: SkillTreeSaveData = ron::from_str(&contents)?;
    info!("Skill tree loaded from {}", path);
    Ok(save_data)
}
