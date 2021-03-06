use bevy::app::AppBuilder;
use bevy::prelude::*;
use crate::core::destroy::DestroyStage;

pub mod destroy;


pub struct CorePlugin {}

impl Plugin for CorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(CoreStage::Last, DestroyStage::Before, SystemStage::parallel());
        app.add_stage_after(DestroyStage::Before, DestroyStage::Prepare, SystemStage::parallel());
        app.add_stage_after(DestroyStage::Prepare, DestroyStage::Destroy, SystemStage::parallel());
        app.add_stage_after(DestroyStage::Destroy, DestroyStage::AfterDestroy, SystemStage::parallel());

        app.add_system_to_stage(DestroyStage::Before, destroy::add_destroy_label_2_children_system.system());
        app.add_system_to_stage(DestroyStage::Destroy, destroy::destroy_system.system());
    }
}