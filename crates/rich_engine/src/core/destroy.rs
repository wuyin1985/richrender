use bevy::prelude::*;
use crate::RenderRunner;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum DestroyStage {
    Prepare,
    Destroy,
    AfterDestroy,
}

pub struct Destroy {}

pub fn destroy_system(
    mut commands: Commands,
    mut query: Query<Entity, With<Destroy>>,
)
{
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}