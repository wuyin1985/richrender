use std::collections::HashMap;
use std::ffi::c_void;
use std::ops::{Deref, DerefMut};

use ash::vk::Handle;
use bevy::ecs::system::LocalState;
use bevy::prelude::{AppBuilder, Local, Plugin, Res, ResMut, Time};
use bevy::transform::TransformSystem;

use crate::{ForwardRenderPass, RenderContext, RenderRunner};
use crate::core::destroy::DestroyStage;
use crate::prelude::*;
use crate::render::CommandBufferList;
use crate::vfx::vfx_resource::{load_vfx_2_device_system, VfxAsset, VfxAssetLoader};
use crate::vfx::vfx_system::{create_vfx_by_req_system, init_vfx_system, play_effect_system, stop_effect_system, update_vfx_system, update_vfx_transform, VfxSystemState};

mod bindings;
mod vfx_resource;
mod vfx_system;

pub use vfx_resource::VfxReq;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum VfxSystemLabel {
    Init,
    Update,
    CreateEntity,
    PlayEffect,
    UpdateTransform,
    StopEffect,
}

pub struct VfxPlugin {}

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(VfxSystemState { inited: false });
        app.init_asset_loader::<VfxAssetLoader>();
        app.add_asset::<VfxAsset>();

        app.add_system_to_stage(
            CoreStage::PreUpdate,
            init_vfx_system.system().label(VfxSystemLabel::Init),
        );

        app.add_system_to_stage(
            CoreStage::PreUpdate,
            create_vfx_by_req_system.system()
                .label(VfxSystemLabel::CreateEntity).after(VfxSystemLabel::Init),
        );

        app.add_system_to_stage(
            RenderStage::ThirdPartyUpload,
            load_vfx_2_device_system.system()
        );

        app.add_system_to_stage(
            CoreStage::Update,
            play_effect_system.system()
                .label(VfxSystemLabel::PlayEffect),
        );

        app.add_system_to_stage(CoreStage::PostUpdate,
                                update_vfx_transform.system()
                                    .label(VfxSystemLabel::UpdateTransform)
                                    .after(TransformSystem::TransformPropagate),
        );

        app.add_system_to_stage(
            RenderStage::PostDraw,
            update_vfx_system.system().label(VfxSystemLabel::Update),
        );

        app.add_system_to_stage(DestroyStage::Prepare,
                                stop_effect_system.system().label(VfxSystemLabel::StopEffect));
    }
}