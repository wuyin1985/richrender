use bevy::prelude::*;
use crate::RenderRunner;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum DestroyStage {
    Before,
    Prepare,
    Destroy,
    AfterDestroy,
}

pub struct Destroy {}

fn add_destroy_recursive(children: &Children, query: &Query<(&Children)>, mut commands: &mut Commands) {
    for c in children.iter() {
        commands.entity(*c).insert(Destroy {});

        if let Ok(cs) = query.get(*c) {
            add_destroy_recursive(cs, query, commands);
        }
    }
}

pub fn add_destroy_label_2_children_system(
    mut commands: Commands,
    mut child_query: Query<(&Children)>,
    mut query: Query<(&Children), With<Destroy>>,
)
{
    for cs in query.iter() {
        add_destroy_recursive(cs, &child_query, &mut commands);
    }
}

pub fn destroy_system(
    mut commands: Commands,
    mut query: Query<Entity, With<Destroy>>,
)
{
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}