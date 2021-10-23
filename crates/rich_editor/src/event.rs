use rich_engine::prelude::Entity;

#[derive(Debug)]
pub enum EditorEvent {
    CreateAsset(String),
    DeleteEntity(Entity),
}